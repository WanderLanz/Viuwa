//! Image sampling, default resize functionality
use super::*;

macro_rules! uninit {
    () => {
        #[allow(invalid_value)]
        unsafe {
            ::core::mem::MaybeUninit::uninit().assume_init()
        }
    };
    ($t:ty) => {
        #[allow(invalid_value)]
        unsafe {
            ::core::mem::MaybeUninit::<$t>::uninit().assume_init()
        }
    };
}

#[derive(Debug, Clone, Copy)]
struct Span {
    pub left: u32,
    pub right: u32,
    pub center: f32,
    pub len: usize,
}
impl Span {
    #[inline]
    pub fn new(out: Weight, ratio: Weight, support: Weight, len: u32) -> Self {
        let center = (out + 0.5) * ratio;
        let left = ((center - support).floor() as u32).min(len - 1);
        let right = ((center + support).ceil() as u32).clamp(left + 1, len);
        let center = center - 0.5;
        Self { left, right, center, len: (right.saturating_sub(left)) as usize }
    }
}

/// Abstracts the bounds used to index into the row buffer and the weights buffer for horizontal sampling
#[derive(Debug, Clone, Copy)]
struct Bound {
    /// The left bound of the row buffer to which the weights apply
    pub start: usize,
    /// The length of the weights buffer that applies to the section of the row buffer specified by start
    pub len: usize,
}
impl Bound {
    #[inline]
    pub fn new(span: Span) -> Self { Self { start: span.left as usize, len: span.len } }
}

#[derive(Debug, Clone, Copy)]
struct Sample {
    pub ratio: (Weight, Weight),
    pub sratio: (Weight, Weight),
    pub support: (Weight, Weight),
    pub max_span: (usize, usize),
}
impl Sample {
    #[inline]
    pub fn new<P: Pixel>(support: Weight, src_dims: (usize, usize), dst_dims: (usize, usize)) -> Self {
        let (w, h) = src_dims;
        let (nw, nh) = dst_dims;
        let ratio = (w as Weight / nw as Weight, h as Weight / nh as Weight);
        let sratio = (ratio.0.max(1.), ratio.1.max(1.));
        let support = (support * sratio.0, support * sratio.1);
        let max_span = (support.0.ceil() as usize * 2 + 1, support.1.ceil() as usize * 2 + 1);
        Self { ratio, sratio, support, max_span }
    }
}

#[inline]
fn fill_weights(kernel: fn(Weight) -> Weight, weights: &mut [Weight], span: Span, sratio: Weight) {
    let mut sum = 0.;
    for (w, i) in weights.iter_mut().zip(span.left..span.right) {
        let coef = kernel((i as Weight - span.center) / sratio);
        *w = coef;
        sum += coef;
    }
    for w in weights.iter_mut() {
        *w /= sum;
    }
}

/// Pre-compute the bounds and weights for horizontal sampling
#[inline]
fn compute_hori_weights<P: Pixel>(
    kernel: fn(Weight) -> Weight,
    Sample { ratio, sratio, support, max_span }: Sample,
    src_width: usize,
    dst_width: usize,
) -> (Vec<Weight>, Vec<Bound>) {
    let mut hwgts = vec![uninit!(Weight); max_span.0 * dst_width];
    let mut bounds = vec![uninit!(Bound); dst_width];
    #[cfg(not(feature = "rayon"))]
    for (outx, (weights, bound)) in hwgts.chunks_exact_mut(max_span.0).zip(bounds.iter_mut()).enumerate() {
        let span = Span::new(outx as Weight, ratio.0, support.0, src_width as u32);
        fill_weights(kernel, &mut weights[..span.len], span, sratio.0);
        *bound = Bound::new(span);
    }
    #[cfg(feature = "rayon")]
    hwgts.par_chunks_exact_mut(max_span.0).zip(bounds.par_iter_mut()).enumerate().for_each(|(outx, (weights, bound))| {
        let span = Span::new(outx as Weight, ratio.0, support.0, src_width as u32);
        fill_weights(kernel, &mut weights[..span.len], span, sratio.0);
        *bound = Bound::new(span);
    });
    (hwgts, bounds)
}

/// Sample the src image into the dst row
fn sample_row<P: Pixel>(
    src: &ImageView<P>,
    buf: &mut [<P::Repr as PixelRepr>::Weights],
    dst_row: &mut [P::Repr],
    hwgts: &[Weight],
    bounds: &[Bound],
    span: Span,
    vwgts: &[Weight],
    hori_max_span: usize,
) {
    buf.fill(<P::Repr as PixelRepr>::Weights::ZERO);
    #[cfg(not(feature = "rayon"))]
    {
        let mut tmp_px = <P::Repr as PixelRepr>::Weights::ZERO;
        let tpx = tmp_px.as_slice_mut();
        for (src_col, buf_px) in src.columns().map(|col| col.skip(span.left as usize).take(span.len)).zip(buf.iter_mut()) {
            for (src_px, coef) in src_col.zip(vwgts.into_iter()) {
                for (d, s) in buf_px.as_slice_mut().iter_mut().zip(src_px.as_slice().into_iter()) {
                    *d += s.weight() * coef;
                }
            }
        }
        for ((Bound { start, len }, weights), dst_px) in
            bounds.iter().zip(hwgts.chunks_exact(hori_max_span)).zip(dst_row.iter_mut())
        {
            tpx.fill(0.);
            let weights = &weights[..*len];
            for (buf_px, coef) in buf[*start..].iter().zip(weights.into_iter()) {
                for (d, s) in tpx.iter_mut().zip(buf_px.as_slice().into_iter()) {
                    *d += s * coef;
                }
            }
            for (d, s) in dst_px.as_slice_mut().iter_mut().zip(tpx.into_iter()) {
                *d = Scalar::scalar(
                    s.round()
                        .clamp(<P::Repr as PixelRepr>::Scalar::MIN.weight(), <P::Repr as PixelRepr>::Scalar::MAX.weight()),
                );
            }
        }
    }
    #[cfg(feature = "rayon")]
    {
        src.par_columns().map(|col| col.skip(span.left as usize).take(span.len)).zip(buf.par_iter_mut()).for_each(
            |(src_col, buf_px)| {
                for (src_px, coef) in src_col.zip(vwgts.into_iter()) {
                    for (d, s) in buf_px.as_slice_mut().iter_mut().zip(src_px.as_slice().into_iter()) {
                        *d += s.weight() * coef;
                    }
                }
            },
        );
        bounds.par_iter().zip(hwgts.par_chunks_exact(hori_max_span)).zip(dst_row.par_iter_mut()).for_each(
            |((Bound { start, len }, weights), dst_px)| {
                let weights = &weights[..*len];
                let mut tmp_px = <P::Repr as PixelRepr>::Weights::ZERO;
                let tpx = tmp_px.as_slice_mut();
                for (buf_px, coef) in buf[*start..].iter().zip(weights.into_iter()) {
                    for (d, s) in tpx.iter_mut().zip(buf_px.as_slice().into_iter()) {
                        *d += s * coef;
                    }
                }
                for (d, s) in dst_px.as_slice_mut().iter_mut().zip(tpx.into_iter()) {
                    *d =
                        Scalar::scalar(s.round().clamp(
                            <P::Repr as PixelRepr>::Scalar::MIN.weight(),
                            <P::Repr as PixelRepr>::Scalar::MAX.weight(),
                        ));
                }
            },
        );
    }
}

/// sample src image into dst image using a given filter, dst image may be uninitialized
pub fn sample<P: Pixel>(Filter { kernel, support }: Filter, src: ImageView<P>, mut dst: ImageViewMut<P>) {
    let sample = Sample::new::<P>(support, src.dimensions(), dst.dimensions());
    let Sample { ratio, sratio, support, max_span } = sample;
    let mut buf = vec![uninit!(<P::Repr as PixelRepr>::Weights); src.width];
    let mut vwgts = vec![uninit!(Weight); max_span.1];
    let (hwgts, bounds) = compute_hori_weights::<P>(kernel, sample, src.width, dst.width);
    for (outy, dst_row) in dst.rows_mut().enumerate() {
        let span = Span::new(outy as f32, ratio.1, support.1, src.height as u32);
        let weights = &mut vwgts[..span.len];
        fill_weights(kernel, weights, span, sratio.1);
        sample_row(&src, buf.as_mut(), dst_row, hwgts.as_ref(), bounds.as_ref(), span, weights, max_span.0);
    }
}

/// sample src image into dst image using a given filter, dst image may be uninitialized.
/// If the source image is larger than the destination image * multiplicity, the source image will be downsampled with a nearest neighbor filter first.
///
/// Currently, this means we give up on some memory savings, but it may significantly improve performance if the source image is very large.
pub fn supersample<P: Pixel>(filter: Filter, src: ImageView<P>, dst: ImageViewMut<P>, multiplicity: f32) {
    // If someone wants to pull out their hair, they can try implement a memory-saving version of this
    let s = Sample::new::<P>(filter.support, src.dimensions(), dst.dimensions());
    if Weight::min(s.ratio.0, s.ratio.1) > multiplicity as Weight {
        let mut buf = unsafe {
            Image::<P>::new_uninit((dst.width as f32 * multiplicity) as usize, (dst.height as f32 * multiplicity) as usize)
        };
        sample::<P>(FILTER_NEAREST, src, buf.view_mut());
        sample::<P>(filter, buf.view(), dst);
    } else {
        sample::<P>(filter, src, dst);
    }
}
