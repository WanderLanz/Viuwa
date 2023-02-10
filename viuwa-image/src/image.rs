//! Image API used for internal image representation.
//! On overflows it will panic to avoid UB and will create a 1x? or ?x1 image if the dimensions are zero, to avoid division by zero.

use ::core::{
    iter::*,
    mem::size_of,
    slice,
    slice::{ChunksExact, ChunksExactMut},
};

use super::*;

const OVERFLOW_PANIC_MSG: &str = "viuwa_image image overflow, viuwa_image does not directly support images larger than 4GB";

pub trait Iter: Iterator + ExactSizeIterator + DoubleEndedIterator {}
pub trait PixelIter<'a, P: Pixel>: Iter<Item = &'a P::Repr> {}
pub trait PixelIterMut<'a, P: Pixel>: Iter<Item = &'a mut P::Repr> {}
impl<I: Iterator + ExactSizeIterator + DoubleEndedIterator> Iter for I {}
impl<'a, P: Pixel, I: Iter<Item = &'a P::Repr>> PixelIter<'a, P> for I {}
impl<'a, P: Pixel, I: Iter<Item = &'a mut P::Repr>> PixelIterMut<'a, P> for I {}
#[cfg(feature = "rayon")]
pub trait ParIter: ParallelIterator + IndexedParallelIterator {}
#[cfg(feature = "rayon")]
pub trait ParPixelIter<'a, P: Pixel>: ParIter<Item = &'a P::Repr> {}
#[cfg(feature = "rayon")]
pub trait ParPixelIterMut<'a, P: Pixel>: ParIter<Item = &'a mut P::Repr> {}
#[cfg(feature = "rayon")]
impl<I: ParallelIterator + IndexedParallelIterator> ParIter for I {}
#[cfg(feature = "rayon")]
impl<'a, P: Pixel, I: ParIter<Item = &'a P::Repr>> ParPixelIter<'a, P> for I {}
#[cfg(feature = "rayon")]
impl<'a, P: Pixel, I: ParIter<Item = &'a mut P::Repr>> ParPixelIterMut<'a, P> for I {}

/// Panics if the given dimensions would overflow the maximum image size.
#[inline]
fn checked_pixels_len<P: Pixel>(width: usize, height: usize) -> usize {
    let len = width.max(1).checked_mul(height.max(1)).expect(OVERFLOW_PANIC_MSG);
    if len.checked_mul(::std::mem::size_of::<P::Repr>()).expect(OVERFLOW_PANIC_MSG) > MAX_IMAGE_SIZE {
        panic!("{}", OVERFLOW_PANIC_MSG);
    }
    len
}
/// Cast a slice of pixel scalars to a slice of pixels, ignoring extra slop (if any)
#[inline]
pub fn pixelate<'a, P: Pixel>(scalars: &'a [P::Scalar]) -> &'a [P::Repr] {
    // SAFETY: the slice is guaranteed to be valid because the given slice is not an owned slice and P::Repr is just an alias for [P::Scalar; P::Repr::CHANNELS]
    unsafe { slice::from_raw_parts(scalars.as_ptr().cast(), scalars.len() / P::Repr::CHANNELS) }
}
/// Cast a mutable slice of pixel scalars to a mutable slice of pixels, ignoring extra slop (if any)
#[inline]
pub fn pixelate_mut<'a, P: Pixel>(scalars: &'a mut [P::Scalar]) -> &'a mut [P::Repr] {
    // SAFETY: the slice is guaranteed to be valid because the given slice is not an owned slice and P::Repr is just an alias for [P::Scalar; P::Repr::CHANNELS]
    unsafe { slice::from_raw_parts_mut(scalars.as_mut_ptr().cast(), scalars.len() / P::Repr::CHANNELS) }
}
/// Cast a slice of pixels to a slice of pixel scalars
#[inline]
pub fn flatten<'a, P: Pixel>(pixels: &'a [P::Repr]) -> &'a [P::Scalar] {
    // SAFETY: the slice is guaranteed to be valid because the given slice is not an owned slice and P::Repr is just an alias for [P::Scalar; P::Repr::CHANNELS]
    unsafe { slice::from_raw_parts(pixels.as_ptr().cast(), pixels.len() * P::Repr::CHANNELS) }
}
/// Cast a mutable slice of pixels to a mutable slice of pixel scalars
#[inline]
pub fn flatten_mut<'a, P: Pixel>(pixels: &'a mut [P::Repr]) -> &'a mut [P::Scalar] {
    // SAFETY: the slice is guaranteed to be valid because the given slice is not an owned slice and P::Repr is just an alias for [P::Scalar; P::Repr::CHANNELS]
    unsafe { slice::from_raw_parts_mut(pixels.as_mut_ptr().cast(), pixels.len() * P::Repr::CHANNELS) }
}
/// Cast a boxed slice of pixel scalars to a boxed slice of pixels
/// # Errors
/// return error if the slice is not a multiple of the pixel size
#[inline]
pub fn pixelate_box<P: Pixel>(scalars: Box<[P::Scalar]>) -> Result<Box<[P::Repr]>, Box<[P::Scalar]>> {
    // if the pixel size is not the same as the scalar size, we need to check if the slice is a multiple of the pixel size
    if size_of::<P::Scalar>() != size_of::<P::Repr>() {
        if scalars.len() % <P::Repr as PixelRepr>::CHANNELS != 0 {
            Err(scalars)
        } else {
            let length = scalars.len() / <P::Repr as PixelRepr>::CHANNELS;
            let box_ptr: *mut [P::Scalar] = Box::into_raw(scalars) as *mut [P::Scalar];
            let ptr: *mut [P::Repr] = unsafe { core::slice::from_raw_parts_mut(box_ptr as *mut P::Repr, length) };
            Ok(unsafe { Box::<[P::Repr]>::from_raw(ptr) })
        }
    } else {
        let box_ptr: *mut [P::Scalar] = Box::into_raw(scalars);
        let ptr: *mut [P::Repr] = box_ptr as *mut [P::Repr];
        Ok(unsafe { Box::<[P::Repr]>::from_raw(ptr) })
    }
}
/// Cast a boxed slice of pixels to a boxed slice of pixel scalars with the width of the image
#[inline]
pub fn flatten_box<P: Pixel>(pixels: Box<[P::Repr]>) -> Box<[P::Scalar]> {
    // SAFETY: the slice is guaranteed to be valid because the given slice is not an owned slice and P::Repr is just an alias for [P::Scalar; P::Repr::CHANNELS]
    let length = pixels.len() * <P::Repr as PixelRepr>::CHANNELS;
    let box_ptr: *mut [P::Repr] = Box::into_raw(pixels) as *mut [P::Repr];
    let ptr: *mut [P::Scalar] = unsafe { core::slice::from_raw_parts_mut(box_ptr as *mut P::Scalar, length) };
    unsafe { Box::<[P::Scalar]>::from_raw(ptr) }
}
/// Rescale the given image dimensions to fit within the new dimensions, maintaining the aspect ratio.
///
/// Returns the new dimensions.
/// # Panics
/// If the dimensions are zero.
/// # Notes
/// Width and height saturates to `u16::MAX` if the new dimensions are too large.
pub fn fit_dimensions(dimensions: (usize, usize), new_dimensions: (usize, usize)) -> (usize, usize) {
    let (w, h) = dimensions;
    let (nw, nh) = new_dimensions;
    let ratio = f64::min(nw as f64 / w as f64, nh as f64 / h as f64);
    let nw = u32::max((w as f64 * ratio).round() as u32, 1);
    let nh = u32::max((h as f64 * ratio).round() as u32, 1);
    if nw > u32::from(u16::MAX) {
        let ratio = u16::MAX as f64 / w as f64;
        (u16::MAX as usize, usize::max((h as f64 * ratio).round() as usize, 1))
    } else if nh > u32::from(u16::MAX) {
        let ratio = u16::MAX as f64 / h as f64;
        (usize::max((w as f64 * ratio).round() as usize, 1), u16::MAX as usize)
    } else {
        (nw as usize, nh as usize)
    }
}
/// Rescale the given image dimensions to fill the new dimensions, maintaining the aspect ratio.
///
/// Returns the new dimensions.
/// # Panics
/// If the dimensions are zero.
/// # Notes
/// Width and height saturates to `u32::MAX` if the new dimensions are too large.
pub fn fill_dimensions(dimensions: (usize, usize), new_dimensions: (usize, usize)) -> (usize, usize) {
    let (w, h) = dimensions;
    let (nw, nh) = new_dimensions;
    let ratio = f64::max(nw as f64 / w as f64, nh as f64 / h as f64);
    let nw = u32::max((w as f64 * ratio).round() as u32, 1);
    let nh = u32::max((h as f64 * ratio).round() as u32, 1);
    if nw > u32::from(u16::MAX) {
        let ratio = u16::MAX as f64 / w as f64;
        (u16::MAX as usize, usize::max((h as f64 * ratio).round() as usize, 1))
    } else if nh > u32::from(u16::MAX) {
        let ratio = u16::MAX as f64 / h as f64;
        (usize::max((w as f64 * ratio).round() as usize, 1), u16::MAX as usize)
    } else {
        (nw as usize, nh as usize)
    }
}

/// Any type that can be used as container for flat image pixel scalars within this library.
/// (e.g. Vec, Box, [u8; 3], etc.)
///
/// Currently only implemented for types that implement `Into<Box<[Scalar]>>`
pub trait Container<P: Pixel>:
    Clone + Into<Box<[<P::Repr as PixelRepr>::Scalar]>> + ::core::ops::Deref<Target = [<P::Repr as PixelRepr>::Scalar]>
{
}
impl<
        P: Pixel,
        C: Clone + Into<Box<[<P::Repr as PixelRepr>::Scalar]>> + ::core::ops::Deref<Target = [<P::Repr as PixelRepr>::Scalar]>,
    > Container<P> for C
{
}

/// Why? because `impl Trait` returns aren't allowed in traits
macro_rules! impl_image_ops {
    (impl$(< $( $lt:tt $( : $clt:tt )? ),+ >)? $name:ident) => {
    impl$(< $( $lt $( : $clt )? ),+ >)? $name$(< $($lt),+ >)? {
        /// Create an [`ImageView`] of the image
        #[inline(always)]
        pub fn view<'b>(&'b self) -> ImageView<'b, P> { ImageView { data: self.data(), width: self.width, height: self.height } }
        /// The number of pixels in each row of the image
        #[inline(always)]
        pub fn width(&self) -> usize { self.width }
        /// The number of pixels in each column of the image
        #[inline(always)]
        pub fn height(&self) -> usize { self.height }
        /// Width and Height of the image
        #[inline(always)]
        pub fn dimensions(&self) -> (usize, usize) { (self.width, self.height) }
        /// Get the pixel at (x, y)
        #[inline]
        pub fn get(&self, x: usize, y: usize) -> Option<&P::Repr> {
            if x < self.width && y < self.height {
                Some(unsafe { self.get_unchecked(x, y) })
            } else {
                None
            }
        }
        /// Get the pixel at (x, y) unchecked
        #[inline]
        pub unsafe fn get_unchecked(&self, x: usize, y: usize) -> &P::Repr {
            self.pixels().get_unchecked(y * self.width + x)
        }
        /// iterate over rows of pixels
        #[inline]
        pub fn rows(&self) -> ChunksExact<P::Repr> { self.pixels().chunks_exact(self.width) }
        /// iterate over rows of pixels in parallel with the width of the image
        #[inline]
        #[cfg(feature = "rayon")]
        pub fn par_rows(&self) -> ParChunksExact<P::Repr> { self.pixels().par_chunks_exact(self.width) }
        /// iterate over columns of pixels with the width of the image
        #[inline]
        pub fn columns(&self) -> impl Iter<Item = impl PixelIter<P>> {
            let mut len = self.pixels().len();
            let width = self.width();
            self.pixels().iter().take(width).map(move |p| {
                let column = unsafe { ::core::slice::from_raw_parts(p as *const P::Repr, len) }.iter().step_by(width);
                len -= 1;
                column
            })
        }
        /// iterate over columns of pixels in parallel with the width of the image
        #[inline]
        #[cfg(feature = "rayon")]
        pub fn par_columns(&self) -> impl ParIter<Item = impl PixelIter<P>> {
            let len = self.pixels().len();
            let width = self.width();
            self.pixels()
                .par_iter()
                .take(width)
                .enumerate()
                .map(move |(i, p)| unsafe { ::core::slice::from_raw_parts(p as *const P::Repr, len - i) }.iter().step_by(width))
        }
        /// Resize the image to the new dimensions, not preserving aspect ratio.
        ///
        /// use [`fit_dimensions`] or [`fill_dimensions`] to preserve aspect ratio.
        #[inline]
        pub fn resize(&self, width: usize, height: usize, filter: &FilterType) -> Image<P> {
            if (width, height) == self.dimensions() {
                return Image { data: self.data().into(), width, height };
            }
            let mut buf = unsafe { Image::new_uninit_unchecked(width, height) };
            sample(filter.filter(), self.view(), buf.view_mut());
            buf
        }
        /// Resize the image to the new dimensions, not preserving aspect ratio,
        ///
        /// if the image is larger than the new dimensions * multiplicity,
        /// it will be downsampled by nearest neighbor first to reduce the amount of work.
        ///
        /// use [`fit_dimensions`] or [`fill_dimensions`] to preserve aspect ratio.
        #[inline]
        pub fn supersize(&self, width: usize, height: usize, filter: &FilterType, multiplicity: f32) -> Image<P> {
            if (width, height) == self.dimensions() {
                return Image { data: self.data().into(), width, height };
            }
            let mut buf = unsafe { Image::new_uninit_unchecked(width, height) };
            supersample(filter.filter(), self.view(), buf.view_mut(), multiplicity);
            buf
        }
        /// [`resize`](Self::resize) using SIMD.
        #[cfg(feature = "fir")]
        pub fn fir_resize(&self, width: usize, height: usize, filter: &FilterType) -> Image<P>
        where
            P::Scalar: CompatScalar,
            P::Repr: CompatPixelRepr,
            P: CompatPixel,
        {
            if (width, height) == self.dimensions() {
                return Image { data: self.data().into(), width, height };
            }
            let mut buf = unsafe { Image::new_uninit_unchecked(width, height) };
            let mut resizer = ::fast_image_resize::Resizer::new(filter.algorithm());
            let v = P::fir_view(self.view());
            let mut mv = P::fir_view_mut(buf.view_mut());
            resizer.resize(&v, &mut mv).expect(concat!("something went wrong: ", module_path!(), "::ImageOps::fir_resize"));
            buf
        }
        /// [`supersize`](Self::supersize) using SIMD.
        #[cfg(feature = "fir")]
        pub fn fir_supersize(&self, width: usize, height: usize, filter: &FilterType, multiplicity: u8) -> Image<P>
        where
            P::Scalar: CompatScalar,
            P::Repr: CompatPixelRepr,
            P: CompatPixel,
        {
            if (width, height) == self.dimensions() {
                return Image { data: self.data().into(), width, height };
            }
            let mut buf = unsafe { Image::new_uninit_unchecked(width, height) };
            let mut resizer = ::fast_image_resize::Resizer::new(filter.ss_algorithm(multiplicity));
            let v = P::fir_view(self.view());
            let mut mv = P::fir_view_mut(buf.view_mut());
            // The only error that can (should) occur is if memory is corrupted, which is a bug.
            resizer.resize(&v, &mut mv).expect(concat!("something went wrong: ", module_path!(), "::ImageOps::fir_supersize"));
            buf
        }
    }
    };
}

/// Why? because `impl Trait` returns aren't allowed in traits yet
macro_rules! impl_image_ops_mut {
    (impl$(< $( $lt:tt $( : $clt:tt )? ),+ >)? $name:ident) => {
    impl$(< $( $lt $( : $clt )? ),+ >)? $name$(< $($lt),+ >)? {
        /// Create an [`ImageView`] of the image
        #[inline]
        pub fn view_mut<'b>(&'b mut self) -> ImageViewMut<'b, P> {
            let (width, height) = (self.width, self.height);
            ImageViewMut { data: self.data_mut(), width, height }
        }
        /// Get the pixel at (x, y)
        #[inline]
        pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut P::Repr> {
            if x < self.width && y < self.height {
                Some(unsafe { self.get_unchecked_mut(x, y) })
            } else {
                None
            }
        }
        /// Get the pixel at (x, y) unchecked
        #[inline]
        pub unsafe fn get_unchecked_mut(&mut self, x: usize, y: usize) -> &mut P::Repr {
            let i = y * self.width + x;
            self.pixels_mut().get_unchecked_mut(i)
        }
        /// iterate over rows of pixels
        #[inline]
        pub fn rows_mut(&mut self) -> ChunksExactMut<P::Repr> {
            let width = self.width();
            self.pixels_mut().chunks_exact_mut(width)
        }
        /// iterate over rows of pixels in parallel with the width of the image
        #[inline]
        #[cfg(feature = "rayon")]
        pub fn par_rows_mut(&mut self) -> ParChunksExactMut<P::Repr> {
            let width = self.width();
            self.pixels_mut().par_chunks_exact_mut(width)
        }
        /// iterate over columns of pixels with the width of the image
        #[inline(always)]
        pub fn columns_mut(&mut self) -> impl Iter<Item = impl PixelIterMut<P>> {
            let mut len = self.pixels().len();
            let width = self.width();
            self.pixels_mut().iter_mut().take(width).map(move |p| {
                let column = unsafe { ::core::slice::from_raw_parts_mut(p as *mut P::Repr, len) }.iter_mut().step_by(width);
                len -= 1;
                column
            })
        }
        /// iterate over columns of pixels in parallel with the width of the image
        #[inline(always)]
        #[cfg(feature = "rayon")]
        pub fn par_columns_mut(&mut self) -> impl ParIter<Item = impl PixelIterMut<P>> {
            let len = self.pixels().len();
            let width = self.width();
            self.pixels_mut().par_iter_mut().take(width).enumerate().map(move |(i, p)| {
                unsafe { ::core::slice::from_raw_parts_mut(p as *mut P::Repr, len - i) }.iter_mut().step_by(width)
            })
        }
    }
    };
}

/// Owned image, use [`ImageView`] for unowned data or [`ImageViewMut`] for unowned mutable data
#[derive(Clone)]
pub struct Image<P: Pixel> {
    /// The image data
    pub(crate) data: Box<[<P::Repr as PixelRepr>::Scalar]>,
    /// The pixel width of the image
    pub(crate) width: usize,
    /// The pixel height of the image
    pub(crate) height: usize,
}
/// Explicitly immutable image view for use with unowned data
#[derive(Clone)]
pub struct ImageView<'a, P: Pixel> {
    /// The image data
    pub(crate) data: &'a [<P::Repr as PixelRepr>::Scalar],
    /// The pixel width of the image
    pub(crate) width: usize,
    /// The pixel height of the image
    pub(crate) height: usize,
}
impl<'a, P: Pixel> From<&'a Image<P>> for ImageView<'a, P> {
    fn from(Image { data, width, height }: &'a Image<P>) -> Self { ImageView { data, width: *width, height: *height } }
}
impl<'a, P: Pixel> From<ImageView<'a, P>> for Image<P> {
    fn from(ImageView { data, width, height }: ImageView<'a, P>) -> Self { Image { data: data.into(), width, height } }
}
/// Explicitly mutable image view for use with unowned data
pub struct ImageViewMut<'a, P: Pixel> {
    /// The image data
    pub(crate) data: &'a mut [<P::Repr as PixelRepr>::Scalar],
    /// The pixel width of the image
    pub(crate) width: usize,
    /// The pixel height of the image
    pub(crate) height: usize,
}
impl<'a, P: Pixel> From<&'a mut Image<P>> for ImageViewMut<'a, P> {
    #[inline(always)]
    fn from(Image { data, width, height }: &'a mut Image<P>) -> Self {
        ImageViewMut { data, width: *width, height: *height }
    }
}
impl<'a, P: Pixel> From<ImageViewMut<'a, P>> for Image<P> {
    fn from(ImageViewMut { data, width, height }: ImageViewMut<'a, P>) -> Self {
        Image { data: (data.as_ref()).into(), width, height }
    }
}
impl<'a, P: Pixel> From<ImageViewMut<'a, P>> for ImageView<'a, P> {
    fn from(ImageViewMut { data, width, height }: ImageViewMut<'a, P>) -> Self { ImageView { data, width, height } }
}
impl<P: Pixel> Image<P> {
    /// Create a new image with default pixel.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            data: flatten_box::<P>(vec![P::DEFAULT; checked_pixels_len::<P>(width, height)].into_boxed_slice()),
            width,
            height,
        }
    }
    /// Create a new image with default pixel unchecked, prefer to use `new` instead for safety.
    /// # Safety
    /// Must not be given a zero for width or height.
    pub unsafe fn new_unchecked(width: usize, height: usize) -> Self {
        Self { data: flatten_box::<P>(vec![P::DEFAULT; width * height].into_boxed_slice()), width, height }
    }
    /// Explicit unitialized constructor, prefer to use `new` instead for safety.
    /// Returns a 1 if given a zero for width or height.
    pub unsafe fn new_uninit(width: usize, height: usize) -> Self {
        let len = checked_pixels_len::<P>(width, height) * P::Repr::CHANNELS;
        Self { data: vec![uninit!(<P::Repr as PixelRepr>::Scalar); len].into(), width, height }
    }
    /// Explicit unitialized and unchecked constructor, prefer to use `new` instead for safety.
    pub unsafe fn new_uninit_unchecked(width: usize, height: usize) -> Self {
        Self {
            data: vec![uninit!(<P::Repr as PixelRepr>::Scalar); width * height * P::Repr::CHANNELS].into(),
            width,
            height,
        }
    }
    /// Create a new image with the given data
    /// # Errors
    /// If the data is not of the correct length (width * height * channels)
    pub fn from_raw<C: Container<P>>(data: C, width: usize, height: usize) -> Result<Self, C> {
        if data.len() == width.max(1) * height.max(1) * P::Repr::CHANNELS {
            Ok(Self { data: data.into(), width, height })
        } else {
            Err(data)
        }
    }
    /// Create a new image with the given data unchecked
    /// # Safety
    /// The data must be of the correct length
    /// (width * height * channels)
    pub unsafe fn from_raw_unchecked<C: Container<P>>(data: C, width: usize, height: usize) -> Self {
        Self { data: data.into(), width, height }
    }
    /// Get the owned data
    #[inline(always)]
    pub fn into_raw(self) -> Box<[P::Scalar]> { self.data }
    /// Get the flattened pixel scalars
    #[inline(always)]
    pub fn data(&self) -> &[P::Scalar] { self.data.as_ref() }
    /// Get the flattened pixels
    #[inline(always)]
    pub fn pixels(&self) -> &[P::Repr] { pixelate::<P>(self.data()) }
    /// Get the flattened mutable pixel scalars
    #[inline(always)]
    pub fn data_mut(&mut self) -> &mut [P::Scalar] { self.data.as_mut() }
    /// Get the flattened mutable pixels
    #[inline]
    pub fn pixels_mut(&mut self) -> &mut [P::Repr] { pixelate_mut::<P>(self.data_mut()) }
}
impl_image_ops!(impl<P: Pixel> Image);
impl_image_ops_mut!(impl<P: Pixel> Image);
impl<'a, P: Pixel> ImageView<'a, P> {
    /// Create a new image view with the given data
    pub fn new(image: &'a Image<P>) -> Self { Self { data: image.data.as_ref(), width: image.width, height: image.height } }
    /// Create a new image view with the given data
    /// # Errors
    /// If the data is not of the correct length (width * height * channels)
    pub fn from_raw(data: &'a [<P::Repr as PixelRepr>::Scalar], width: usize, height: usize) -> Option<Self> {
        if data.len() == width.max(1) * height.max(1) * P::Repr::CHANNELS {
            Some(Self { data, width, height })
        } else {
            None
        }
    }
    /// Create a new image view with the given data unchecked
    /// # Safety
    /// The data must be of the correct length
    /// (width * height * channels)
    pub unsafe fn from_raw_unchecked(data: &'a [<P::Repr as PixelRepr>::Scalar], width: usize, height: usize) -> Self {
        Self { data, width, height }
    }
    #[inline(always)]
    pub fn data(&self) -> &'a [P::Scalar] { self.data }
    /// Get the flattened pixels
    #[inline]
    pub fn pixels(&self) -> &'a [P::Repr] { pixelate::<P>(self.data) }
}
impl_image_ops!(impl<'a, P: Pixel> ImageView);
impl<'a, P: Pixel> ImageViewMut<'a, P> {
    /// Create a new image view with the given data
    pub fn new(image: &'a mut Image<P>) -> Self {
        Self { data: image.data.as_mut(), width: image.width, height: image.height }
    }
    /// Create a new image view with the given data
    /// # Errors
    /// If the data is not of the correct length (width * height * channels)
    pub fn from_raw(data: &'a mut [<P::Repr as PixelRepr>::Scalar], width: usize, height: usize) -> Option<Self> {
        if data.len() == width.max(1) * height.max(1) * P::Repr::CHANNELS {
            Some(Self { data, width, height })
        } else {
            None
        }
    }
    /// Create a new image view with the given data unchecked
    /// # Safety
    /// The data must be of the correct length
    /// (width * height * channels)
    pub unsafe fn from_raw_unchecked(data: &'a mut [<P::Repr as PixelRepr>::Scalar], width: usize, height: usize) -> Self {
        Self { data, width, height }
    }
    /// Get the flattened pixel scalars
    #[inline(always)]
    pub fn data(&self) -> &[P::Scalar] { self.data }
    /// Get the flattened pixels
    #[inline]
    pub fn pixels(&self) -> &[P::Repr] { pixelate::<P>(self.data) }
    /// Get the flattened mutable pixel scalars
    #[inline(always)]
    pub fn data_mut(&mut self) -> &mut [P::Scalar] { self.data }
    /// Get the flattened mutable pixels
    #[inline]
    pub fn pixels_mut(&mut self) -> &mut [P::Repr] { pixelate_mut::<P>(self.data_mut()) }
}
impl_image_ops!(impl<'a, P: Pixel> ImageViewMut);
impl_image_ops_mut!(impl<'a, P: Pixel> ImageViewMut);
impl<P: Pixel> Default for Image<P> {
    fn default() -> Self { Self::new(1, 1) }
}

macro_rules! impl_Index {
    () => {
        type Output = P::Repr;
        #[inline(always)]
        fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
            assert!(x < self.width && y < self.height);
            unsafe { self.pixels().get_unchecked(y * self.width + x) }
        }
    };
}
macro_rules! impl_IndexMut {
    () => {
        #[inline(always)]
        fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
            assert!(x < self.width && y < self.height);
            let i = y * self.width + x;
            unsafe { self.pixels_mut().get_unchecked_mut(i) }
        }
    };
}

impl<P: Pixel> ::core::ops::Index<(usize, usize)> for Image<P> {
    impl_Index!();
}
impl<'a, P: Pixel> ::core::ops::Index<(usize, usize)> for ImageView<'a, P> {
    impl_Index!();
}
impl<'a, P: Pixel> ::core::ops::Index<(usize, usize)> for ImageViewMut<'a, P> {
    impl_Index!();
}
impl<P: Pixel> ::core::ops::IndexMut<(usize, usize)> for Image<P> {
    impl_IndexMut!();
}
impl<'a, P: Pixel> ::core::ops::IndexMut<(usize, usize)> for ImageViewMut<'a, P> {
    impl_IndexMut!();
}

#[cfg(feature = "image")]
mod compat_image {
    use super::*;
    impl<
            T: Scalar,
            R: PixelRepr<Scalar = T>,
            P: ::image::Pixel<Subpixel = T> + Pixel<Repr = R>,
            C: Clone + ::core::ops::Deref<Target = [T]> + From<Box<[T]>>,
        > From<Image<P>> for ::image::ImageBuffer<P, C>
    {
        #[inline(always)]
        fn from(Image { data, width, height, .. }: Image<P>) -> Self {
            Self::from_raw(width as u32, height as u32, From::from(data)).unwrap()
        }
    }
    impl<
            T: Scalar,
            R: PixelRepr<Scalar = T>,
            P: ::image::Pixel<Subpixel = T> + Pixel<Repr = R>,
            C: Clone + ::core::ops::Deref<Target = [T]> + Into<Box<[T]>>,
        > From<::image::ImageBuffer<P, C>> for Image<P>
    {
        #[inline(always)]
        fn from(image: ::image::ImageBuffer<P, C>) -> Self {
            let (w, h) = image.dimensions();
            Self { data: image.into_raw().into(), width: w as usize, height: h as usize }
        }
    }
    impl<
            'a,
            T: Scalar,
            R: PixelRepr<Scalar = T>,
            P: ::image::Pixel<Subpixel = T> + Pixel<Repr = R>,
            C: Clone + ::core::ops::Deref<Target = [T]>,
        > From<&'a ::image::ImageBuffer<P, C>> for ImageView<'a, P>
    {
        #[inline(always)]
        fn from(image: &'a ::image::ImageBuffer<P, C>) -> Self {
            let (w, h) = image.dimensions();
            Self { data: image.as_ref(), width: w as usize, height: h as usize }
        }
    }
    impl<
            'a,
            T: Scalar,
            R: PixelRepr<Scalar = T>,
            P: ::image::Pixel<Subpixel = T> + Pixel<Repr = R>,
            C: Clone + ::core::ops::Deref<Target = [T]> + ::core::ops::DerefMut,
        > From<&'a mut ::image::ImageBuffer<P, C>> for ImageViewMut<'a, P>
    {
        #[inline(always)]
        fn from(image: &'a mut ::image::ImageBuffer<P, C>) -> Self {
            let (w, h) = image.dimensions();
            Self { data: image.as_mut(), width: w as usize, height: h as usize }
        }
    }
}
#[cfg(feature = "image")]
pub use self::compat_image::*;
