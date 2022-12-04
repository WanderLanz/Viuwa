use super::*;
#[derive(Debug, Clone, Copy)]
pub struct XY<T: Copy> {
    pub x: T,
    pub y: T,
}
impl<T: Copy> XY<T> {
    #[inline]
    fn new(x: T, y: T) -> Self { Self { x, y } }
}
#[derive(Debug, Clone, Copy)]
pub struct SamplerImage<Data: Sized> {
    /// The image data
    pub data: Data,
    /// The width of the image in pixels
    pub width: usize,
    /// The width of the image in pixels
    pub height: usize,
    /// The length of a row of the image in raw pixel data
    pub row_len: usize,
    /// The length of a col of the image in raw pixel data
    pub col_len: usize,
}
impl<'a, P: Pixel> From<&'a ImageBuffer<P, Vec<u8>>> for SamplerImage<&'a [u8]> {
    #[inline(always)]
    fn from(img: &'a ImageBuffer<P, Vec<u8>>) -> Self {
        Self {
            data: img.as_raw(),
            width: img.width() as usize,
            height: img.height() as usize,
            row_len: img.width() as usize * P::CHANNELS,
            col_len: img.height() as usize * P::CHANNELS,
        }
    }
}
impl<'a, P: Pixel> From<&'a mut ImageBuffer<P, Vec<u8>>> for SamplerImage<&'a mut [u8]> {
    #[inline(always)]
    fn from(img: &'a mut ImageBuffer<P, Vec<u8>>) -> Self {
        let (width, height) = img.dimensions();
        Self {
            data: core::ops::DerefMut::deref_mut(img),
            width: width as usize,
            height: height as usize,
            row_len: width as usize * P::CHANNELS,
            col_len: height as usize * P::CHANNELS,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct SamplerSpan {
    pub left: u32,
    pub right: u32,
    pub center: f32,
}
impl SamplerSpan {
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

// macros to get around the borrow checker complaining
macro_rules! get_hspan {
    ($sampler:ident, $outx:ident) => {
        SamplerSpan::new($outx as f32, $sampler.ratio.x, $sampler.support.x, $sampler.src.width as u32)
    };
}
macro_rules! get_vspan {
    ($sampler:ident, $outy:ident) => {
        SamplerSpan::new($outy as f32, $sampler.ratio.y, $sampler.support.y, $sampler.src.height as u32)
    };
}
macro_rules! fill_wgts {
    ($sampler:ident, $wgts:ident, $span:ident, $sratio: expr) => {
        let mut sum = 0.0;
        for (weight, i) in $wgts.iter_mut().zip($span.left..$span.right) {
            let coef = ($sampler.kernel)((i as f32 - $span.center) / $sratio);
            *weight = coef;
            sum += coef;
        }
        for w in $wgts.iter_mut() {
            *w /= sum;
        }
    };
}
macro_rules! fill_hwgts {
    ($sampler:ident, $wgts:ident, $span:ident) => {
        fill_wgts!($sampler, $wgts, $span, $sampler.ratio.x);
    };
}
macro_rules! fill_vwgts {
    ($sampler:ident, $wgts:ident, $span:ident) => {
        fill_wgts!($sampler, $wgts, $span, $sampler.ratio.y);
    };
}

#[inline(always)]
/// an approximation of the memory needed for the buffers when vertically sampling into a row buffer before horizontally sampling
fn vertical_first_mem<P: Pixel>(src: &SamplerImage<&[u8]>, dst: &SamplerImage<&mut [u8]>, max_span: &XY<usize>) -> usize {
    (max_span.x * dst.width + src.row_len + max_span.y) * core::mem::size_of::<f32>()
        + dst.width * core::mem::size_of::<[usize; 2]>()
}
#[inline(always)]
/// an approximation of the memory needed for the buffers when horizontally sampling into a column buffer before vertically sampling
fn horizontal_first_mem<P: Pixel>(src: &SamplerImage<&[u8]>, dst: &SamplerImage<&mut [u8]>, max_span: &XY<usize>) -> usize {
    (max_span.y * dst.height + src.col_len + max_span.x) * core::mem::size_of::<f32>()
        + dst.height * core::mem::size_of::<[usize; 2]>()
}
/// unitialized vec for write-before-read optimization
/// # Safety
/// - Data in the buffer must be initialized before read to avoid undefined behavior.
/// - For any type `T` that includes a reference or pointer, reading the data before initializing will cause undefined behavior.
/// - For any unbounded type `T` that does not include a reference or pointer, e.g. ```f32,u8,i64,...```, reading the data before initializing will cause unexpected behavior.
macro_rules! unvec {
    ($len:expr) => {
        unsafe {
            let mut v = Vec::with_capacity($len);
            v.set_len($len);
            v
        }
    };
}

#[derive(Debug)]
pub struct Sampler<'a, P: Pixel> {
    /// The kernel function
    pub kernel: fn(f32) -> f32,
    /// The image to sample from
    pub src: SamplerImage<&'a [u8]>,
    /// The image to sample into
    pub dst: SamplerImage<&'a mut [u8]>,
    /// The buffer to store the row or column of pixels
    buf: Vec<f32>,
    /// The horizontal weights
    hwgts: Vec<f32>,
    /// The vertical weights
    vwgts: Vec<f32>,
    /// The bounds of the horizontal or vertical weights buffer
    bounds: Vec<[usize; 2]>,
    /// width (or height) ratios
    pub ratio: XY<f32>,
    /// The filter radius ratios
    pub sratio: XY<f32>,
    /// The filter radii
    pub support: XY<f32>,
    /// the maximum filter radii
    pub max_span: XY<usize>,
    /// a boolean indicating whether or not to sample vertically first (true) or horizontally first (false) for the best memory usage
    pub vertical_first: bool,
    _phantom: core::marker::PhantomData<P>,
}
#[cfg(feature = "rayon")]
unsafe impl<'a, P: Pixel> Send for Sampler<'a, P> {}
#[cfg(feature = "rayon")]
unsafe impl<'a, P: Pixel> Sync for Sampler<'a, P> {}
impl<'a, P: Pixel> Sampler<'a, P> {
    pub fn new(filter: &Filter, src: &'a ImageBuffer<P, Vec<u8>>, dst: &'a mut ImageBuffer<P, Vec<u8>>) -> Self {
        let src = SamplerImage::from(src);
        let dst = SamplerImage::from(dst);
        let ratio = XY::new(src.width as f32 / dst.width as f32, src.height as f32 / dst.height as f32);
        let sratio = XY::new(ratio.x.max(1.), ratio.y.max(1.));
        let support = XY::new(filter.support * sratio.x, filter.support * sratio.y);
        let max_span = XY::new(support.x.ceil() as usize * 2 + 1, support.y.ceil() as usize * 2 + 1);
        let vf = vertical_first_mem::<P>(&src, &dst, &max_span);
        let hf = horizontal_first_mem::<P>(&src, &dst, &max_span);
        // prefer vertical first at the cost of some memory in order to avoid potential caching and paging slowdowns
        // REVIEW: Is this a sufficient way to do this, and does this even actually help after compiler optimizations and for large images?
        let vertical_first = if vf > 4096 { (vf as f64 / hf as f64) < 1.5 } else { true };
        let (buf, hwgts, vwgts, bounds) = if vertical_first {
            debug!("Sampler::new", "vertical first: {}B", vf);
            (unvec!(src.row_len), unvec!(max_span.x * dst.width), unvec!(max_span.y), unvec!(dst.width))
        } else {
            debug!("Sampler::new", "horizontal first: {}B", hf);
            (unvec!(src.col_len), unvec!(max_span.x), unvec!(max_span.y * dst.height), unvec!(dst.height))
        };
        Self {
            kernel: filter.kernel,
            src,
            dst,
            buf,
            hwgts,
            vwgts,
            bounds,
            ratio,
            sratio,
            support,
            max_span,
            vertical_first,
            _phantom: core::marker::PhantomData,
        }
    }
    pub fn sample(self) {
        let mut tmp_px = vec![0_f32; P::CHANNELS];
        if self.vertical_first {
            self.sample_vertical_first(&mut tmp_px);
        } else {
            self.sample_horizontal_first(&mut tmp_px);
        }
    }
    #[inline(always)]
    /// fill the horizontal weights buffer with the weights for every column in dst
    fn fill_hwgts_buf(&mut self) {
        trace!("Sampler::fill_hwgts_buf");
        #[cfg(not(feature = "rayon"))]
        for (outx, (weights, bound)) in self.hwgts.chunks_exact_mut(self.max_span.x).zip(self.bounds.iter_mut()).enumerate()
        {
            let span = get_hspan!(self, outx);
            let len = span.len();
            let weights = &mut weights[..len];
            fill_hwgts!(self, weights, span);
            *bound = [span.left as usize * P::CHANNELS, span.len()];
        }
        #[cfg(feature = "rayon")]
        self.hwgts.par_chunks_exact_mut(self.max_span.x).zip(self.bounds.par_iter_mut()).enumerate().for_each(
            |(outx, (weights, bound))| {
                let span = get_hspan!(self, outx);
                let len = span.len();
                let weights = &mut weights[..len];
                fill_hwgts!(self, weights, span);
                *bound = [span.left as usize * P::CHANNELS, span.len()];
            },
        )
    }
    #[inline(always)]
    /// fill the vertical weights buffer with the weights for every row in dst
    fn fill_vwgts_buf(&mut self) {
        trace!("Sampler::fill_vwgts_buf");
        #[cfg(not(feature = "rayon"))]
        for (outy, (weights, bound)) in self.vwgts.chunks_exact_mut(self.max_span.y).zip(self.bounds.iter_mut()).enumerate()
        {
            let span = get_vspan!(self, outy);
            let len = span.len();
            let weights = &mut weights[..len];
            fill_vwgts!(self, weights, span);
            *bound = [span.left as usize * P::CHANNELS, len];
        }
        #[cfg(feature = "rayon")]
        self.vwgts.par_chunks_exact_mut(self.max_span.y).zip(self.bounds.par_iter_mut()).enumerate().for_each(
            |(outy, (weights, bound))| {
                let span = get_vspan!(self, outy);
                let len = span.len();
                let weights = &mut weights[..len];
                fill_vwgts!(self, weights, span);
                *bound = [span.left as usize * P::CHANNELS, len];
            },
        );
    }
    #[inline(always)]
    fn fill_col_buf(&mut self, outx: usize) {
        let span = get_hspan!(self, outx);
        let weights = &mut self.hwgts[..span.len()];
        fill_hwgts!(self, weights, span);
        // vertical sample src into col buffer
        self.buf.fill(0.);
        #[cfg(not(feature = "rayon"))]
        for (src_off, buf_px) in (span.left as usize * P::CHANNELS..self.src.data.len())
            .step_by(self.src.row_len)
            .zip(self.buf.chunks_exact_mut(P::CHANNELS))
        {
            for (src_px, coef) in self.src.data[src_off..].chunks_exact(P::CHANNELS).zip(weights.iter()) {
                for (d, s) in buf_px.iter_mut().zip(src_px.iter()) {
                    *d += *s as f32 * *coef;
                }
            }
        }
        #[cfg(feature = "rayon")]
        (span.left as usize * P::CHANNELS..self.src.data.len())
            .into_par_iter()
            .step_by(self.src.row_len)
            .zip(self.buf.par_chunks_exact_mut(P::CHANNELS))
            .for_each(|(src_off, buf_px)| {
                for (src_px, coef) in self.src.data[src_off..].chunks_exact(P::CHANNELS).zip(weights.iter()) {
                    for (d, s) in buf_px.iter_mut().zip(src_px.iter()) {
                        *d += *s as f32 * *coef;
                    }
                }
            });
    }
    #[inline(always)]
    fn fill_row_buf(&mut self, outy: usize) {
        let span = get_vspan!(self, outy);
        let weights = &mut self.vwgts[..span.len()];
        fill_vwgts!(self, weights, span);
        // vertical sample src into row buffer
        self.buf.fill(0.);
        let src_off = span.left as usize * self.src.row_len;
        #[cfg(not(feature = "rayon"))]
        for (src_off, buf_px) in (src_off..).step_by(P::CHANNELS).zip(self.buf.chunks_exact_mut(P::CHANNELS)) {
            for (src_off, coef) in (src_off..).step_by(self.src.row_len).zip(weights.iter()) {
                let src_px = &self.src.data[src_off..src_off + P::CHANNELS];
                for (d, s) in buf_px.iter_mut().zip(src_px.iter()) {
                    *d += *s as f32 * *coef;
                }
            }
        }
        #[cfg(feature = "rayon")]
        (src_off..src_off + self.src.row_len)
            .into_par_iter()
            .step_by(P::CHANNELS)
            .zip(self.buf.par_chunks_exact_mut(P::CHANNELS))
            .for_each(|(src_off, buf_px)| {
                for (src_off, coef) in (src_off..).step_by(self.src.row_len).zip(weights.iter()) {
                    let src_px = &self.src.data[src_off..src_off + P::CHANNELS];
                    for (d, s) in buf_px.iter_mut().zip(src_px.iter()) {
                        *d += *s as f32 * *coef;
                    }
                }
            });
    }
    /// vertical-first sampling (row buffer and pre-computed horizontal weights)
    pub fn sample_vertical_first(mut self, _tpx: &mut [f32]) {
        trace!("Sampler::sample_vertical_first");
        self.fill_hwgts_buf();
        for (outy, dst_off) in (0..self.dst.height).zip((0_usize..).step_by(self.dst.row_len)) {
            self.fill_row_buf(outy);
            #[cfg(not(feature = "rayon"))]
            for (([buf_off, clen], weights), dst_px) in self
                .bounds
                .iter()
                .zip(self.hwgts.chunks_exact_mut(self.max_span.x))
                .zip(self.dst.data[dst_off..dst_off + self.dst.row_len].chunks_exact_mut(P::CHANNELS))
            {
                _tpx.fill(0.);
                let weights = &mut weights[..*clen];
                for (buf_px, coef) in self.buf[*buf_off..].chunks_exact(P::CHANNELS).zip(weights.iter()) {
                    for (d, s) in _tpx.iter_mut().zip(buf_px.iter()) {
                        *d += *s * coef;
                    }
                }
                for (d, s) in dst_px.iter_mut().zip(_tpx.iter()) {
                    *d = s.round().clamp(u8::MIN as f32, u8::MAX as f32) as u8;
                }
            }
            #[cfg(feature = "rayon")]
            self.bounds
                .par_iter()
                .zip(self.hwgts.par_chunks_exact_mut(self.max_span.x))
                .zip(self.dst.data[dst_off..dst_off + self.dst.row_len].par_chunks_exact_mut(P::CHANNELS))
                .for_each(|(([buf_off, clen], weights), dst_px)| {
                    let weights = &mut weights[..*clen];
                    let mut tmp_px_max = [0_f32; 4];
                    let tpx = &mut tmp_px_max[..P::CHANNELS];
                    for (buf_px, coef) in self.buf[*buf_off..].chunks_exact(P::CHANNELS).zip(weights.iter()) {
                        for (d, s) in tpx.iter_mut().zip(buf_px.iter()) {
                            *d += *s * coef;
                        }
                    }
                    for (d, s) in dst_px.iter_mut().zip(tpx.iter()) {
                        *d = s.round().clamp(u8::MIN as f32, u8::MAX as f32) as u8;
                    }
                });
        }
    }
    /// horizontal-first sampling (column buffer and pre-computed vertical weights)
    pub fn sample_horizontal_first(mut self, _tpx: &mut [f32]) {
        trace!("Sampler::sample_horizontal_first");
        self.fill_vwgts_buf();
        for (outx, dst_off) in (0..self.dst.width).zip((0_usize..).step_by(P::CHANNELS)) {
            self.fill_col_buf(outx);
            #[cfg(not(feature = "rayon"))]
            for (([buf_off, clen], weights), dst_off) in self
                .bounds
                .iter()
                .zip(self.vwgts.chunks_exact_mut(self.max_span.y))
                .zip((dst_off..).step_by(self.dst.row_len))
            {
                let dst_px = &mut self.dst.data[dst_off..dst_off + P::CHANNELS];
                _tpx.fill(0.);
                let weights = &mut weights[..*clen];
                for (buf_px, coef) in self.buf[*buf_off..].chunks_exact(P::CHANNELS).zip(weights.iter()) {
                    for (d, s) in _tpx.iter_mut().zip(buf_px.iter()) {
                        *d += *s * coef;
                    }
                }
                for (d, s) in dst_px.iter_mut().zip(_tpx.iter()) {
                    *d = s.round().clamp(u8::MIN as f32, u8::MAX as f32) as u8;
                }
            }
            #[cfg(feature = "rayon")]
            self.bounds
                .par_iter()
                .zip(self.vwgts.par_chunks_exact_mut(self.max_span.y))
                .zip(self.dst.data[dst_off..].par_chunks_exact_mut(P::CHANNELS).step_by(self.dst.width))
                .for_each(|(([buf_off, clen], weights), dst_px)| {
                    let weights = &mut weights[..*clen];
                    let mut tmp_px_max = [0_f32; 4];
                    let tpx = &mut tmp_px_max[..P::CHANNELS];
                    for (buf_px, coef) in self.buf[*buf_off..].chunks_exact(P::CHANNELS).zip(weights.iter()) {
                        for (d, s) in tpx.iter_mut().zip(buf_px.iter()) {
                            *d += *s * coef;
                        }
                    }
                    for (d, s) in dst_px.iter_mut().zip(tpx.iter()) {
                        *d = s.round().clamp(u8::MIN as f32, u8::MAX as f32) as u8;
                    }
                });
        }
    }
}

/// sample src image into dst image using a given filter, optimized for memory usage, although should be only slightly slower
pub fn sample<P: Pixel>(
    src: &ImageBuffer<P, Vec<P::Subpixel>>,
    dst: &mut ImageBuffer<P, Vec<P::Subpixel>>,
    filter: &Filter,
) {
    trace!("sample");
    Sampler::new(filter, src, dst).sample();
}
