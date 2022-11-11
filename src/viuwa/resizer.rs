//! Uses the same algorithms as the `image` crate, but with optional parallelism and completely avoiding memory allocations
//! (might have been optimized out before, but we make it explicit).
use super::*;

#[inline]
fn sinc(t: f32) -> f32 {
    if t == 0.0 {
        1.0
    } else {
        let a = t * std::f32::consts::PI;
        a.sin() / a
    }
}

/// A specific static filter with a kernel fn and a support radius
#[derive(Clone)]
pub struct Filter {
    kernel: fn(f32) -> f32,
    support: f32,
}
fn nearest(_: f32) -> f32 { 1.0 }
pub static FILTER_NEAREST: Filter = Filter {
    kernel: nearest,
    support: 0.0,
};
fn triangle(x: f32) -> f32 {
    if x.abs() < 1.0 {
        1.0 - x.abs()
    } else {
        0.0
    }
}
pub static FILTER_TRIANGLE: Filter = Filter {
    kernel: triangle,
    support: 1.0,
};
fn catmull_rom(x: f32) -> f32 {
    let a = x.abs();
    let k = if a < 1.0 {
        9.0 * a.powi(3) - 15.0 * a.powi(2) + 6.0
    } else if a < 2.0 {
        -3.0 * a.powi(3) + 15.0 * a.powi(2) - 24.0 * a + 12.0
    } else {
        0.0
    };
    k / 6.0
}
pub static FILTER_CATMULL_ROM: Filter = Filter {
    kernel: catmull_rom,
    support: 2.0,
};
fn gaussian(x: f32) -> f32 { 0.7978846 * (-x.powi(2) / 0.5).exp() }
pub static FILTER_GAUSSIAN: Filter = Filter {
    kernel: gaussian,
    support: 3.0,
};
fn lanczos3(x: f32) -> f32 {
    let t = 3.0;
    if x.abs() < t {
        sinc(x) * sinc(x / t)
    } else {
        0.0
    }
}
pub static FILTER_LANCZOS3: Filter = Filter {
    kernel: lanczos3,
    support: 3.0,
};

/// Dynamic filter type
#[derive(Debug, Clone, Copy)]
pub enum FilterType {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}
impl FilterType {
    #[inline]
    pub fn filter(&self) -> &Filter {
        match self {
            FilterType::Nearest => &FILTER_NEAREST,
            FilterType::Triangle => &FILTER_TRIANGLE,
            FilterType::CatmullRom => &FILTER_CATMULL_ROM,
            FilterType::Gaussian => &FILTER_GAUSSIAN,
            FilterType::Lanczos3 => &FILTER_LANCZOS3,
        }
    }
    #[inline]
    pub fn cycle(&self) -> FilterType {
        match self {
            FilterType::Nearest => FilterType::Triangle,
            FilterType::Triangle => FilterType::CatmullRom,
            FilterType::CatmullRom => FilterType::Gaussian,
            FilterType::Gaussian => FilterType::Lanczos3,
            FilterType::Lanczos3 => FilterType::Nearest,
        }
    }
}

pub enum ResizerPixels {
    Rgb(ImageBuffer<Rgb<u8>, Vec<u8>>),
    Luma(ImageBuffer<Luma<u8>, Vec<u8>>),
    None,
}
impl Default for ResizerPixels {
    #[inline]
    fn default() -> Self { ResizerPixels::None }
}
impl From<ImageBuffer<Rgb<u8>, Vec<u8>>> for ResizerPixels {
    #[inline]
    fn from(value: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Self { ResizerPixels::Rgb(value) }
}
impl From<ImageBuffer<Luma<u8>, Vec<u8>>> for ResizerPixels {
    #[inline]
    fn from(value: ImageBuffer<Luma<u8>, Vec<u8>>) -> Self { ResizerPixels::Luma(value) }
}
impl ResizerPixels {
    #[inline(always)]
    pub fn new() -> Self { ResizerPixels::None }
    #[inline(always)]
    pub unsafe fn take(&mut self) -> Self { core::mem::take(self) }
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        match self {
            ResizerPixels::Rgb(pixels) => pixels.as_raw(),
            ResizerPixels::Luma(pixels) => pixels.as_raw(),
            ResizerPixels::None => &[],
        }
    }
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        use core::ops::DerefMut;
        match self {
            ResizerPixels::Rgb(pixels) => pixels.deref_mut(),
            ResizerPixels::Luma(pixels) => pixels.deref_mut(),
            ResizerPixels::None => &mut [],
        }
    }
    #[inline]
    pub unsafe fn take_vec(&mut self) -> Vec<u8> {
        match self {
            ResizerPixels::Rgb(b) => core::mem::take(b).into_vec(),
            ResizerPixels::Luma(b) => core::mem::take(b).into_vec(),
            ResizerPixels::None => unreachable!(crate::err_msg!("ResizerPixels::take_vec: None")),
        }
    }
    #[inline]
    pub unsafe fn insert_vec(&mut self, vec: Vec<u8>, width: u32, height: u32) {
        match self {
            ResizerPixels::Rgb(b) => core::mem::drop(core::mem::replace(
                b,
                ImageBuffer::from_vec(width, height, vec).expect(crate::err_msg!("ResizerPixels::insert_vec: invalid vec")),
            )),
            ResizerPixels::Luma(b) => core::mem::drop(core::mem::replace(
                b,
                ImageBuffer::from_vec(width, height, vec).expect(crate::err_msg!("ResizerPixels::insert_vec: invalid vec")),
            )),
            ResizerPixels::None => unreachable!(crate::err_msg!("ResizerPixels::insert_vec: None")),
        };
    }
    #[inline]
    pub fn is_rgb(&self) -> bool {
        match self {
            ResizerPixels::Rgb(_) => true,
            _ => false,
        }
    }
    #[inline]
    pub fn is_luma(&self) -> bool {
        match self {
            ResizerPixels::Luma(_) => true,
            _ => false,
        }
    }
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            ResizerPixels::Rgb(pixels) => pixels.dimensions(),
            ResizerPixels::Luma(pixels) => pixels.dimensions(),
            ResizerPixels::None => (0, 0),
        }
    }
    #[inline]
    pub fn as_luma8(&self) -> &ImageBuffer<Luma<u8>, Vec<u8>> {
        match self {
            ResizerPixels::Luma(pixels) => pixels,
            _ => unreachable!(crate::err_msg!("ResizerPixels::as_luma8: pixels are not luma8")),
        }
    }
    #[inline]
    pub fn as_rgb8(&self) -> &ImageBuffer<Rgb<u8>, Vec<u8>> {
        match self {
            ResizerPixels::Rgb(pixels) => pixels,
            _ => unreachable!(crate::err_msg!("ResizerPixels::as_rgb8: pixels are not rgb8")),
        }
    }
    #[inline]
    pub fn channels(&self) -> usize {
        match self {
            ResizerPixels::Rgb(_) => 3,
            ResizerPixels::Luma(_) => 1,
            ResizerPixels::None => 0,
        }
    }
}

pub struct Resizer {
    filter: FilterType,
    /// The original image data
    orig: Vec<u8>,
    /// The original image width and height
    osize: (u32, u32),
    /// The number of channels (1 for Luma or 3 for Rgb)
    channels: usize,
    /// Intermediate data after vertical resizing
    vert: Vec<f32>,
    /// Resized image after horizontal resizing
    resized: ResizerPixels,
    /// The resized image's width and height
    size: (u32, u32),
}

impl Resizer {
    /// Create a Resizer with the given image data, and resize it to the given size.
    pub fn from_rgb(orig: ImageBuffer<Rgb<u8>, Vec<u8>>, filter: &FilterType, (w, h): (u32, u32)) -> Self {
        let (ow, oh) = orig.dimensions();
        let (w, h) = _bounded_size((ow, oh), (w, h));
        let mut orig = orig.into_vec();
        orig.shrink_to_fit();
        let mut ret = Self {
            filter: *filter,
            orig,
            osize: (ow, oh),
            channels: 3,
            vert: Vec::with_capacity((w * h * 3) as usize),
            resized: ResizerPixels::Rgb(ImageBuffer::new(w, h)),
            size: (0, 0),
        };
        ret.resize(w, h);
        ret
    }
    /// Create a Resizer with the given image data, and resize it to the given size.
    pub fn from_luma(orig: ImageBuffer<Luma<u8>, Vec<u8>>, filter: &FilterType, (w, h): (u32, u32)) -> Self {
        let (ow, oh) = orig.dimensions();
        let (w, h) = _bounded_size((ow, oh), (w, h));
        let mut orig = orig.into_vec();
        orig.shrink_to_fit();
        let mut ret = Self {
            filter: *filter,
            orig,
            osize: (ow, oh),
            channels: 1,
            vert: Vec::with_capacity((w * h * 3) as usize),
            resized: ResizerPixels::Luma(ImageBuffer::new(w, h)),
            size: (0, 0),
        };
        ret.resize(w, h);
        ret
    }
    /// Take the resized image as a Vec<u8>.
    /// # Safety
    /// Changing the size of the Vec<u8> will invalidate it when given back with [`Resizer::insert_vec`]
    /// # Panics
    /// Panics if the ResizerPixels is None
    #[inline(always)]
    pub unsafe fn take_vec(&mut self) -> Vec<u8> { self.resized.take_vec() }
    /// Insert a Vec<u8> as the resized image.
    /// # Panics
    /// Panics if the Vec<u8> is not the same size as the current size of the image or ResizerPixels::None.
    #[inline(always)]
    pub unsafe fn insert_vec(&mut self, buf: Vec<u8>) { self.resized.insert_vec(buf, self.size.0, self.size.1); }
    /// Set the filter type to use for resizing
    #[inline(always)]
    pub fn filter(&mut self, filter: FilterType) { self.filter = filter; }
    /// Cycle through the available filter types
    #[inline(always)]
    pub fn cycle_filter(&mut self) { self.filter = self.filter.cycle(); }
    /// Get a reference to the resized image
    #[inline(always)]
    pub fn resized(&self) -> &ResizerPixels { &self.resized }
    /// Resize the image to the given size, preserving the aspect ratio
    pub fn resize(&mut self, nw: u32, nh: u32) {
        let (nw, nh) = _bounded_size(self.osize, (nw, nh));
        self.size = (nw, nh);
        let mut r = unsafe { self.take_vec() };
        if self.osize == self.size {
            // Image is very small and fits within screen, just copy it
            r.resize(self.orig.len(), 0_u8);
            r.copy_from_slice(&self.orig);
            unsafe { self.insert_vec(r) };
        } else {
            self.vert.clear();
            self.vert.resize((self.channels as u32 * self.osize.0 * nh) as usize, 0_f32);
            r.resize((self.channels as u32 * nw * nh) as usize, 0_u8);
            unsafe { self.insert_vec(r) };
            // Always resize both vertically and horizontally because we preserve aspect ratio (nw < w, nh < h)
            self.vsample();
            self.hsample();
        }
    }
    #[cfg(feature = "rayon")]
    fn vsample(&mut self) {
        let channels = self.channels;
        let nh = self.size.1;
        let (w, h) = self.osize;
        let row_stride = channels * w as usize;
        let filter = self.filter.filter();

        let ratio = h as f32 / nh as f32;
        let sratio = if ratio < 1.0 { 1.0 } else { ratio };
        let src_support = filter.support * sratio;
        ndarray::ArrayViewMut3::<f32>::from_shape([nh as usize, w as usize, channels], &mut self.vert)
            .expect(crate::err_msg!("Resizer::vsample: invalid shape"))
            .outer_iter_mut()
            .into_par_iter()
            .enumerate()
            .for_each_with(&self.orig, |orig, (outy, mut row)| {
                let inputy = (outy as f32 + 0.5) * ratio;
                let left = ((inputy - src_support).floor() as i64).clamp(0, (h - 1) as i64) as u32;
                let right = ((inputy + src_support).ceil() as i64).clamp((left + 1) as i64, h as i64) as u32;
                let inputy = inputy - 0.5;
                let mut weights = Vec::with_capacity(right.saturating_sub(left) as usize);
                let mut sum = 0.0;
                for i in left..right {
                    let w = (filter.kernel)((i as f32 - inputy) / sratio);
                    weights.push((i as usize * row_stride, w));
                    sum += w;
                }
                weights.iter_mut().for_each(|(_, w)| *w /= sum);
                (0..row_stride)
                    .step_by(channels)
                    .zip(row.outer_iter_mut())
                    .for_each(|(x, mut np)| {
                        weights.iter().for_each(|(y, w)| {
                            let p = y + x;
                            np.iter_mut()
                                .zip(unsafe { orig.get_unchecked(p..p + channels) }.iter())
                                .for_each(|(t, o)| *t += *o as f32 * w)
                        });
                    });
            });
    }
    #[cfg(not(feature = "rayon"))]
    fn vsample(&mut self) {
        let channels = self.channels;
        let nh = self.size.1;
        let (w, h) = self.osize;
        let row_stride = channels * w as usize;
        let filter = self.filter.filter();

        let ratio = h as f32 / nh as f32;
        let sratio = if ratio < 1.0 { 1.0 } else { ratio };
        let src_support = filter.support * sratio;
        ndarray::ArrayViewMut3::<f32>::from_shape([nh as usize, w as usize, channels], &mut self.vert)
            .expect(crate::err_msg!("Resizer::vsample: invalid shape"))
            .outer_iter_mut()
            .enumerate()
            .for_each(|(outy, mut row)| {
                let inputy = (outy as f32 + 0.5) * ratio;
                let left = ((inputy - src_support).floor() as i64).clamp(0, (h - 1) as i64) as u32;
                let right = ((inputy + src_support).ceil() as i64).clamp((left + 1) as i64, h as i64) as u32;
                let inputy = inputy - 0.5;
                let mut weights = Vec::with_capacity(right.saturating_sub(left) as usize);
                let mut sum = 0.0;
                for i in left..right {
                    let w = (filter.kernel)((i as f32 - inputy) / sratio);
                    weights.push((i as usize * row_stride, w));
                    sum += w;
                }
                weights.iter_mut().for_each(|(_, w)| *w /= sum);
                (0..row_stride)
                    .step_by(channels)
                    .zip(row.outer_iter_mut())
                    .for_each(|(x, mut np)| {
                        weights.iter().for_each(|(y, w)| {
                            let p = y + x;
                            np.iter_mut()
                                .zip(unsafe { self.orig.get_unchecked(p..p + channels) }.iter())
                                .for_each(|(t, o)| *t += *o as f32 * w)
                        });
                    });
            });
    }
    #[cfg(feature = "rayon")]
    fn hsample(&mut self) {
        let channels: usize = self.channels;
        let nw = self.size.0;
        let (w, h) = (self.osize.0, self.size.1);
        let row_stride = channels * w as usize;
        let end = row_stride * h as usize;
        let filter: &Filter = self.filter.filter();

        let max: f32 = u8::MAX as f32;
        let min: f32 = u8::MIN as f32;
        let ratio = w as f32 / nw as f32;
        let sratio = if ratio < 1.0 { 1.0 } else { ratio };
        let src_support = filter.support * sratio;
        ndarray::ArrayViewMut3::from_shape(
            [h as usize, nw as usize, channels].strides([channels * nw as usize, channels, 1]),
            self.resized.as_mut_slice(),
        )
        .expect(crate::err_msg!("Resizer::hsample: invalid shape"))
        .axis_iter_mut(Axis(1))
        .into_par_iter()
        .enumerate()
        .for_each_with(&self.vert, |orig, (outx, mut col)| {
            let inputx = (outx as f32 + 0.5) * ratio;
            let left = ((inputx - src_support).floor() as i64).clamp(0, (w - 1) as i64) as u32;
            let right = ((inputx + src_support).ceil() as i64).clamp((left + 1) as i64, w as i64) as u32;
            let inputx = inputx - 0.5;
            let mut weights = Vec::with_capacity(right.saturating_sub(left) as usize);
            let mut sum = 0.0;
            for i in left..right {
                let w = (filter.kernel)((i as f32 - inputx) / sratio);
                weights.push((i as usize * channels, w));
                sum += w;
            }
            weights.iter_mut().for_each(|(_, w)| *w /= sum);
            let mut t = vec![0_f32; channels];
            let t = t.as_mut_slice();
            (0..end)
                .step_by(row_stride)
                .zip(col.outer_iter_mut())
                .for_each(|(y, mut np)| {
                    t.fill(0_f32);
                    weights.iter().for_each(|(x, w)| {
                        let p = y + x;
                        t.iter_mut()
                            .zip(unsafe { orig.get_unchecked(p..p + channels) }.iter())
                            .for_each(|(t, o)| *t += *o * w)
                    });
                    np.iter_mut()
                        .zip(t.iter())
                        .for_each(|(np, t)| *np = t.clamp(min, max).round() as u8);
                });
        });
    }
    #[cfg(not(feature = "rayon"))]
    fn hsample(&mut self) {
        let channels: usize = self.channels;
        let nw = self.size.0;
        let (w, h) = (self.osize.0, self.size.1);
        let row_stride = channels * w as usize;
        let end = row_stride * h as usize;
        let filter: &Filter = self.filter.filter();

        let max: f32 = u8::MAX as f32;
        let min: f32 = u8::MIN as f32;
        let ratio = w as f32 / nw as f32;
        let sratio = if ratio < 1.0 { 1.0 } else { ratio };
        let src_support = filter.support * sratio;
        ndarray::ArrayViewMut3::from_shape(
            [h as usize, nw as usize, channels].strides([channels * nw as usize, channels, 1]),
            self.resized.as_mut_slice(),
        )
        .expect(crate::err_msg!("Resizer::hsample: invalid shape"))
        .axis_iter_mut(Axis(1))
        .enumerate()
        .for_each(|(outx, mut col)| {
            let inputx = (outx as f32 + 0.5) * ratio;
            let left = ((inputx - src_support).floor() as i64).clamp(0, (w - 1) as i64) as u32;
            let right = ((inputx + src_support).ceil() as i64).clamp((left + 1) as i64, w as i64) as u32;
            let inputx = inputx - 0.5;
            let mut weights = Vec::with_capacity(right.saturating_sub(left) as usize);
            let mut sum = 0.0;
            for i in left..right {
                let w = (filter.kernel)((i as f32 - inputx) / sratio);
                weights.push((i as usize * channels, w));
                sum += w;
            }
            weights.iter_mut().for_each(|(_, w)| *w /= sum);
            let mut t = vec![0_f32; channels];
            let t = t.as_mut_slice();
            (0..end)
                .step_by(row_stride)
                .zip(col.outer_iter_mut())
                .for_each(|(y, mut np)| {
                    t.fill(0_f32);
                    weights.iter().for_each(|(x, w)| {
                        let p = y + x;
                        t.iter_mut()
                            .zip(unsafe { self.vert.get_unchecked(p..p + channels) }.iter())
                            .for_each(|(t, o)| *t += *o * w)
                    });
                    np.iter_mut()
                        .zip(t.iter())
                        .for_each(|(np, t)| *np = t.clamp(min, max).round() as u8);
                });
        });
    }
}
fn _bounded_size((w, h): (u32, u32), (nw, nh): (u32, u32)) -> (u32, u32) {
    // No need to resize to a larger image
    if nw > w && nh > h {
        return (w, h);
    }
    let wratio = nw as f64 / w as f64;
    let hratio = nh as f64 / h as f64;
    let ratio = f64::min(wratio, hratio);
    let nw = u64::max((w as f64 * ratio).round() as u64, 1);
    let nh = u64::max((h as f64 * ratio).round() as u64, 1);
    if nw > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / w as f64;
        (u32::MAX, u32::max((h as f64 * ratio).round() as u32, 1))
    } else if nh > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / h as f64;
        (u32::max((w as f64 * ratio).round() as u32, 1), u32::MAX)
    } else {
        (nw as u32, nh as u32)
    }
}
