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
        trace!("Resizer::resize");
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
            r.resize(self.orig.len(), uninit!());
            r.copy_from_slice(&self.orig);
            if let Err(_) = self.insert_vec(r) {
                panic!("Resizer::resize: Buffer has run out of space");
            };
        } else {
            r.resize((P::CHANNELS as u32 * nw * nh) as usize, uninit!());
            if let Err(_) = self.insert_vec(r) {
                panic!("Resizer::resize: Buffer has run out of space");
            };
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
