//! Uses the same algorithms as the `image` crate, but with optional parallelism and completely avoiding memory allocations
//! (might have been optimized out before, but we make it explicit).
#[cfg(feature = "fir")]
use fast_image_resize as fir;

use super::*;
mod filtertype;
pub use filtertype::*;

#[cfg(not(feature = "fir"))]
mod sampler;
#[cfg(not(feature = "fir"))]
use sampler::*;

fn bounded_size((w, h): (u32, u32), (nw, nh): (u32, u32)) -> (u32, u32) {
    // No need to resize to a larger image
    if nw >= w && nh >= h {
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

pub struct Resizer<P: Pixel> {
    filter: FilterType,
    /// Keeping track of the last resize to avoid unnecessary work
    last_filter: FilterType,
    /// The original image data
    orig: ImageBuffer<P, Vec<u8>>,
    /// Resized image after horizontal resizing
    resized: ImageBuffer<P, Vec<u8>>,
    /// The resized image's width and height
    size: (u32, u32),
    #[cfg(feature = "fir")]
    resizer: fir::Resizer,
}

impl<P: Pixel> Resizer<P> {
    pub fn new(orig: ImageBuffer<P, Vec<u8>>, filter: &FilterType, (w, h): (u32, u32)) -> Self {
        let (w, h) = bounded_size(orig.dimensions(), (w, h));
        let buf_len = (w * h) as usize * P::CHANNELS;
        let mut buf = Vec::with_capacity(buf_len);
        unsafe { buf.set_len(buf_len) };
        let mut ret = Self {
            filter: *filter,
            last_filter: *filter,
            orig,
            resized: ImageBuffer::from_raw(w, h, buf).unwrap(),
            size: (0, 0),
            #[cfg(feature = "fir")]
            resizer: fir::Resizer::new(filter.filter()),
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
    pub unsafe fn take_vec(&mut self) -> Vec<u8> { core::mem::take(&mut self.resized).into_raw() }
    /// Insert a Vec<u8> as the resized image.
    /// # Panics
    /// Panics if the Vec<u8> is not the same size as the current size of the image or ResizerPixels::None.
    #[inline(always)]
    pub fn insert_vec(&mut self, buf: Vec<u8>) -> Result<()> {
        let Some(src) = ImageBuffer::from_raw(self.size.0, self.size.1, buf) else {
            return Err(anyhow!("Resizer::insert_vec: Invalid buffer size"));
        };
        let _ = core::mem::replace(&mut self.resized, src);
        Ok(())
    }
    /// Set the filter type to use for resizing
    #[cfg(not(feature = "fir"))]
    #[inline(always)]
    pub fn filter(&mut self, filter: FilterType) { self.filter = filter; }
    /// Set the filter type to use for resizing
    #[cfg(feature = "fir")]
    #[inline(always)]
    pub fn filter(&mut self, filter: FilterType) {
        self.filter = filter;
        self.resizer.algorithm = filter.filter();
    }
    #[inline]
    /// Cycle through the available filter types
    pub fn cycle_filter(&mut self) { self.filter(self.filter.cycle()); }
    /// Get a reference to the resized image
    #[inline(always)]
    pub fn resized(&self) -> &ImageBuffer<P, Vec<u8>> { &self.resized }
    /// Resize the image to the given size, preserving the aspect ratio
    pub fn resize(&mut self, nw: u32, nh: u32) {
        crate::timer!("Resizer::resize");
        let (nw, nh) = bounded_size(self.orig.dimensions(), (nw, nh));
        if self.size == (nw, nh) && self.last_filter == self.filter {
            return;
        }
        self.size = (nw, nh);
        self.last_filter = self.filter;
        let mut r = unsafe { self.take_vec() };
        if self.orig.dimensions() == self.size {
            #[allow(invalid_value)]
            // Image is very small and fits within screen, just copy it
            r.resize(self.orig.len(), crate::uninit!());
            r.copy_from_slice(&self.orig);
            let _ = self.insert_vec(r);
        } else {
            r.resize((P::CHANNELS as u32 * nw * nh) as usize, 0_u8);
            let _ = self.insert_vec(r);
            // Always resize both vertically and horizontally because we preserve aspect ratio (nw < w, nh < h)
            #[cfg(not(feature = "fir"))]
            {
                sample(&self.orig, &mut self.resized, self.filter.filter())
            }
            #[cfg(feature = "fir")]
            {
                // Always resize both vertically and horizontally because we preserve aspect ratio (nw < w, nh < h)
                let oimg = P::fir_view_image(&self.orig).unwrap();
                let mut nimg = P::fir_view_image_mut(&mut self.resized).unwrap();
                self.resizer.resize(&oimg, &mut nimg).unwrap();
            }
        }
    }
}
// #[cfg(not(feature = "fir"))]
// impl<P: Pixel> Resizer<P> {
// fn vsample(&mut self) {
//     let channels = self.channels;
//     let nh = self.size.1;
//     let (w, h) = self.osize;
//     let row_stride = channels * w as usize;
//     let filter = self.filter.filter();

//     let ratio = h as f32 / nh as f32;
//     let sratio = ratio.max(1.);
//     let src_support = filter.support * sratio;

//     // pre allocate all the weights
//     let max_span = src_support.ceil() as usize * 2 + 1;
//     let wgs_len = max_span * nh as usize;
//     let mut wgs: Vec<(usize, f32)> = Vec::with_capacity(wgs_len);
//     unsafe { wgs.set_len(wgs_len) };
//     let wgs = wgs.as_slice();
//     let wgs_ptr = wgs.as_ptr() as usize;

//     ndarray::ArrayViewMut3::<f32>::from_shape([nh as usize, w as usize, channels], &mut self.vert)
//         .expect(concat!(module_path!(), "Resizer::vsample: invalid shape"))
//         .outer_iter_mut()
//         .into_par_iter()
//         .enumerate()
//         .for_each_with(&self.orig, |orig, (outy, mut row)| {
//             let inputy = (outy as f32 + 0.5) * ratio;
//             let left = ((inputy - src_support).floor() as i64).clamp(0, (h - 1) as i64) as u32;
//             let right = ((inputy + src_support).ceil() as i64).clamp((left + 1) as i64, h as i64) as u32;
//             let inputy = inputy - 0.5;
//             let weights_left = outy * max_span;
//             let weights_right = weights_left + (right - left) as usize;
//             let weights = unsafe {
//                 std::slice::from_raw_parts_mut(wgs_ptr as *mut (usize, f32), wgs_len)
//                     .get_unchecked_mut(weights_left..weights_right)
//             };
//             let mut sum = 0.0;
//             for (s, i) in weights.iter_mut().zip(left..right) {
//                 let w = (filter.kernel)((i as f32 - inputy) / sratio);
//                 *s = (i as usize * row_stride, w);
//                 sum += w;
//             }
//             weights.iter_mut().for_each(|(_, w)| *w /= sum);
//             (0..row_stride)
//                 .step_by(channels)
//                 .zip(row.outer_iter_mut())
//                 .for_each(|(x, mut np)| {
//                     weights.iter().for_each(|(y, w)| {
//                         let p = y + x;
//                         np.iter_mut()
//                             .zip(unsafe { orig.get_unchecked(p..p + channels) }.iter())
//                             .for_each(|(t, o)| *t += *o as f32 * w)
//                     });
//                 });
//         });
// }
// fn hsample(&mut self) {
//     let channels: usize = self.channels;
//     let nw = self.size.0;
//     let (w, h) = (self.osize.0, self.size.1);
//     let row_stride = channels * w as usize;
//     let end = row_stride * h as usize;
//     let filter: &Filter = self.filter.filter();

//     let max: f32 = u8::MAX as f32;
//     let min: f32 = u8::MIN as f32;
//     let ratio = w as f32 / nw as f32;
//     let sratio = ratio.max(1.);
//     let src_support = filter.support * sratio;

//     // pre allocate all the weights
//     let max_span = src_support.ceil() as usize * 2 + 1;
//     let wgs_len = max_span * nw as usize;
//     let mut wgs: Vec<(usize, f32)> = Vec::with_capacity(wgs_len);
//     unsafe { wgs.set_len(wgs_len) };
//     let wgs = wgs.as_slice();
//     let wgs_ptr = wgs.as_ptr() as usize;

//     ndarray::ArrayViewMut3::from_shape(
//         [h as usize, nw as usize, channels].strides([channels * nw as usize, channels, 1]),
//         self.resized.as_mut_slice(),
//     )
//     .expect(concat!(module_path!(), "Resizer::hsample: invalid shape"))
//     .axis_iter_mut(Axis(1))
//     .into_par_iter()
//     .enumerate()
//     .for_each_with(&self.vert, |orig, (outx, mut col)| {
//         let inputx = (outx as f32 + 0.5) * ratio;
//         let left = ((inputx - src_support).floor() as i64).clamp(0, (w - 1) as i64) as u32;
//         let right = ((inputx + src_support).ceil() as i64).clamp((left + 1) as i64, w as i64) as u32;
//         let inputx = inputx - 0.5;
//         let weights_left = outx * max_span;
//         let weights_right = weights_left + (right - left) as usize;
//         let weights = unsafe {
//             std::slice::from_raw_parts_mut(wgs_ptr as *mut (usize, f32), wgs_len)
//                 .get_unchecked_mut(weights_left..weights_right)
//         };
//         let mut sum = 0.0;
//         for (s, i) in weights.iter_mut().zip(left..right) {
//             let w = (filter.kernel)((i as f32 - inputx) / sratio);
//             *s = (i as usize * channels, w);
//             sum += w;
//         }
//         weights.iter_mut().for_each(|(_, w)| *w /= sum);
//         let mut t = vec![0_f32; channels];
//         let t = t.as_mut_slice();
//         (0..end)
//             .step_by(row_stride)
//             .zip(col.outer_iter_mut())
//             .for_each(|(y, mut np)| {
//                 t.fill(0_f32);
//                 weights.iter().for_each(|(x, w)| {
//                     let p = y + x;
//                     t.iter_mut()
//                         .zip(unsafe { orig.get_unchecked(p..p + channels) }.iter())
//                         .for_each(|(t, o)| *t += *o * w)
//                 });
//                 np.iter_mut()
//                     .zip(t.iter())
//                     .for_each(|(np, t)| *np = t.clamp(min, max).round() as u8);
//             });
//     });
// }
// }
