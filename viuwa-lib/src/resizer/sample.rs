use super::*;

#[derive(Debug, Clone, Copy)]
struct Span {
    pub left: u32,
    pub right: u32,
    pub center: f32,
}
impl Span {
    #[inline(always)]
    pub fn len(&self) -> usize { (self.right.saturating_sub(self.left)) as usize }
    #[inline(always)]
    pub fn new(out: f32, ratio: f32, support: f32, len: u32) -> Self {
        let center = (out + 0.5) * ratio;
        let left = ((center - support).floor() as u32).min(len - 1);
        let right = ((center + support).ceil() as u32).clamp(left + 1, len);
        let center = center - 0.5;
        Self { left, right, center }
    }
}

macro_rules! fill_wgts {
    ($kernel:ident, $wgts:ident, $span:ident, $sratio: expr) => {
        let mut sum = 0.0;
        for (weight, i) in $wgts.iter_mut().zip($span.left..$span.right) {
            let coef = ($kernel)((i as f32 - $span.center) / $sratio);
            *weight = coef;
            sum += coef;
        }
        for w in $wgts.iter_mut() {
            *w /= sum;
        }
    };
}

/// Abstracts the bounds used to index into the row buffer and the weights buffer for horizontal sampling
#[derive(Debug, Clone, Copy)]
struct Bound {
    /// The left bound of the row buffer to which the weights apply
    pub start: usize,
    /// The length of the weights buffer that applies to the section of the row buffer specified by start
    pub len: usize,
}

/// sample src image into dst image using a given filter, dst image may be uninitialized
#[instrument(skip_all, level = "trace")]
pub fn sample<P: RawPixel>(Filter { kernel, support }: Filter, src: ImageView<P>, mut dst: ImageViewMut<P>) {
    let src_row_len = src.width * P::Repr::CHANNELS;
    let dst_row_len = dst.width * P::Repr::CHANNELS;
    let ratio = (src.width as f32 / dst.width as f32, src.height as f32 / dst.height as f32);
    let sratio = (ratio.0.max(1.), ratio.1.max(1.));
    let support = (support * sratio.0, support * sratio.1);
    let max_span = (support.0.ceil() as usize * 2 + 1, support.1.ceil() as usize * 2 + 1);
    debug!(target: "Sampler::new","{}B vs full {}B", (max_span.0 * dst.width + src_row_len + max_span.1) * core::mem::size_of::<f32>()
        + dst.width * core::mem::size_of::<[usize; 2]>(), src_row_len * dst.height * core::mem::size_of::<f32>() + dst_row_len * dst.height);
    let mut buf = vec![uninit!(Weight); src_row_len];
    let mut hwgts = vec![uninit!(Weight); max_span.0 * dst.width];
    let mut vwgts = vec![uninit!(Weight); max_span.1];
    let mut bounds = vec![uninit!(Bound); dst.width];
    let mut tmp_px = vec![0_f32; P::Repr::CHANNELS];
    #[cfg(not(feature = "rayon"))]
    for (outx, (weights, bound)) in hwgts.chunks_exact_mut(max_span.x).zip(bounds.iter_mut()).enumerate() {
        let span = Span::new(outx as f32, ratio.0, support.0, src.width as u32);
        let len = span.len();
        let weights = &mut weights[..len];
        fill_wgts!(kernel, weights, span, ratio.x);
        *bound = [span.left as usize * P::CHANNELS, span.len()];
    }
    #[cfg(feature = "rayon")]
    hwgts.par_chunks_exact_mut(max_span.0).zip(bounds.par_iter_mut()).enumerate().for_each(|(outx, (weights, bound))| {
        let span = Span::new(outx as f32, ratio.0, support.0, src.width as u32);
        let len = span.len();
        let weights = &mut weights[..len];
        fill_wgts!(kernel, weights, span, ratio.0);
        *bound = Bound { start: span.left as usize * P::Repr::CHANNELS, len };
    });
    for (outy, dst_row) in dst.rows_mut().enumerate() {
        let span = Span::new(outy as f32, ratio.1, support.1, src.height as u32);
        let len = span.len();
        let weights = &mut vwgts[..len];
        fill_wgts!(kernel, weights, span, ratio.1);
        buf.fill(0.);
        #[cfg(not(feature = "rayon"))]
        {
            for (src_col, buf_px) in
                src.columns().map(|col| col.skip(span.left as usize).take(len)).zip(buf.chunks_exact_mut(P::Repr::CHANNELS))
            {
                for (src_px, coef) in src_col.zip(weights.iter()) {
                    for (d, s) in buf_px.iter_mut().zip(src_px.iter()) {
                        *d += *s as f32 * *coef;
                    }
                }
            }
            for (([buf_off, clen], weights), dst_px) in
                bounds.iter().zip(hwgts.chunks_exact_mut(max_span.0)).zip(dst_row.iter_mut())
            {
                tmp_px.fill(0.);
                let weights = &mut weights[..*clen];
                for (buf_px, coef) in buf[*buf_off..].chunks_exact(P::CHANNELS).zip(weights.iter()) {
                    for (d, s) in tmp_px.iter_mut().zip(buf_px.iter()) {
                        *d += *s * coef;
                    }
                }
                for (d, s) in dst_px.iter_mut().zip(tmp_px.iter()) {
                    *d = s.round().clamp(u8::MIN as f32, u8::MAX as f32) as u8;
                }
            }
        }
        #[cfg(feature = "rayon")]
        {
            src.par_columns()
                .map(|col| col.skip(span.left as usize).take(len))
                .zip(buf.par_chunks_exact_mut(P::Repr::CHANNELS))
                .for_each(|(src_col, buf_px)| {
                    for (src_px, coef) in src_col.zip(weights.iter()) {
                        for (d, s) in buf_px.iter_mut().zip(src_px.into_iter()) {
                            *d += s.weight() * *coef;
                        }
                    }
                });
            bounds.par_iter().zip(hwgts.par_chunks_exact_mut(max_span.0)).zip(dst_row.par_iter_mut()).for_each(
                |((Bound { start, len }, weights), dst_px)| {
                    let weights = &mut weights[..*len];
                    let mut tmp_px_max = [0_f32; 4];
                    let tpx = &mut tmp_px_max[..P::Repr::CHANNELS];
                    for (buf_px, coef) in buf[*start..].chunks_exact(P::Repr::CHANNELS).zip(weights.iter()) {
                        for (d, s) in tpx.iter_mut().zip(buf_px.iter()) {
                            *d += *s * coef;
                        }
                    }
                    for (d, s) in dst_px.as_mut().iter_mut().zip(tpx.iter()) {
                        *d = Scalar::scalar(s.round().clamp(u8::MIN as f32, u8::MAX as f32));
                    }
                },
            );
        }
    }
}
