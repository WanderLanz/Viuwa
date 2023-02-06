//! Image API used for internal image representation.
//! On overflows it will panic to avoid UB and will create a 1x? or ?x1 image if the dimensions are zero, to avoid division by zero.

use ::core::{
    iter::*,
    slice::{ChunksExact, ChunksExactMut, Iter, IterMut},
};

use super::*;

const OVERFLOW_PANIC_MSG: &str = "viuwa_image image overflow, viuwa_image does not directly support images larger than 4GB";
const CAST_PANIC_MSG: &str = "viuwa_image image byte cast error, please report this to us with your platform information";

/// Panics if the given dimensions would overflow the maximum image size.
#[inline]
fn checked_pixels_len<P: Pixel>(width: usize, height: usize) -> usize {
    let len = width.max(1).checked_mul(height.max(1)).expect(OVERFLOW_PANIC_MSG);
    if len.checked_mul(::std::mem::size_of::<P::Repr>()).expect(OVERFLOW_PANIC_MSG) > MAX_IMAGE_SIZE {
        panic!("{}", OVERFLOW_PANIC_MSG);
    }
    len
}
/// Cast a slice of scalars to a slice of pixel representations
#[inline]
pub(crate) fn pixelate<'a, P: Pixel>(data: &'a [<P::Repr as PixelRepr>::Scalar]) -> &'a [P::Repr] {
    ::bytemuck::try_cast_slice::<<P::Repr as PixelRepr>::Scalar, P::Repr>(data).expect(CAST_PANIC_MSG)
}
/// Cast a slice of scalars to a slice of pixel representations
#[inline]
pub(crate) fn pixelate_mut<'a, P: Pixel>(data: &'a mut [<P::Repr as PixelRepr>::Scalar]) -> &'a mut [P::Repr] {
    ::bytemuck::try_cast_slice_mut::<<P::Repr as PixelRepr>::Scalar, P::Repr>(data).expect(CAST_PANIC_MSG)
}
/// Cast a boxed slice of pixel representations to a boxed slice of scalars
#[inline]
pub(crate) fn unpixelate_box<P: Pixel>(pixels: Box<[P::Repr]>) -> Box<[<P::Repr as PixelRepr>::Scalar]> {
    ::bytemuck::try_cast_slice_box::<P::Repr, <P::Repr as PixelRepr>::Scalar>(pixels)
        .unwrap_or_else(|(e, _)| panic!("{} : {}", CAST_PANIC_MSG, e))
}

/// Squeeze the given image dimensions to fit within the given bounds, maintaining the aspect ratio.
/// Returns the new width and height (which may be the same as the original).
/// # Panics
/// If the dimensions are zero.
/// # Notes
/// Saturates to `u32::MAX` if the new dimensions are too large.
fn squeeze_dimensions((w, h): (usize, usize), (nw, nh): (usize, usize)) -> (usize, usize) {
    let ratio = f64::min(nw as f64 / w as f64, nh as f64 / h as f64);
    let nw = u64::max((w as f64 * ratio).round() as u64, 1);
    let nh = u64::max((h as f64 * ratio).round() as u64, 1);
    if nw > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / w as f64;
        (u32::MAX as usize, usize::max((h as f64 * ratio).round() as usize, 1))
    } else if nh > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / h as f64;
        (usize::max((w as f64 * ratio).round() as usize, 1), u32::MAX as usize)
    } else {
        (nw as usize, nh as usize)
    }
}

/// Any type that can be used as container for flat image pixel data within this library.
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

/// Image API with a pixel type `P`, used for internal image representation.
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
/// Explicitly mutable image view for use with unowned data
pub struct ImageViewMut<'a, P: Pixel> {
    /// The image data
    pub(crate) data: &'a mut [<P::Repr as PixelRepr>::Scalar],
    /// The pixel width of the image
    pub(crate) width: usize,
    /// The pixel height of the image
    pub(crate) height: usize,
}

impl<P: Pixel> Image<P> {
    /// Create a new image with default pixel.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            data: unpixelate_box::<P>(vec![P::DEFAULT; checked_pixels_len::<P>(width, height)].into_boxed_slice()),
            width,
            height,
        }
    }
    /// Create a new image with default pixel unchecked, prefer to use `new` instead for safety.
    /// # Safety
    /// Must not be given a zero for width or height.
    pub unsafe fn new_unchecked(width: usize, height: usize) -> Self {
        Self { data: unpixelate_box::<P>(vec![P::DEFAULT; width * height].into_boxed_slice()), width, height }
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
    /// Create a new image view
    #[inline(always)]
    pub fn view(&self) -> ImageView<P> { ImageView::new(self) }
    /// Create a new mutable image view
    #[inline(always)]
    pub fn view_mut(&mut self) -> ImageViewMut<P> { ImageViewMut::new(self) }
    /// Get the raw data of the image
    #[inline(always)]
    pub fn into_raw(self) -> Box<[<P::Repr as PixelRepr>::Scalar]> { self.data }
}
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
}
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
}
impl<P: Pixel> Default for Image<P> {
    fn default() -> Self { Self::new(1, 1) }
}

fn _map_column<P: Pixel>(((skip, start), (len, step)): ((usize, &P::Repr), (usize, usize))) -> StepBy<Iter<P::Repr>> {
    unsafe { ::core::slice::from_raw_parts(start as *const P::Repr, len - skip) }.iter().step_by(step)
}
fn _map_column_mut<P: Pixel>(
    ((skip, start), (len, step)): ((usize, &mut P::Repr), (usize, usize)),
) -> StepBy<IterMut<P::Repr>> {
    unsafe { ::core::slice::from_raw_parts_mut(start as *mut P::Repr, len - skip) }.iter_mut().step_by(step)
}

pub type ColumnIter<'a, P> = StepBy<Iter<'a, <P as Pixel>::Repr>>;
pub type ColumnIterMut<'a, P> = StepBy<IterMut<'a, <P as Pixel>::Repr>>;
pub type ColumnsIter<'a, P> = Map<
    Zip<Enumerate<Iter<'a, <P as Pixel>::Repr>>, Repeat<(usize, usize)>>,
    fn(((usize, &'a <P as Pixel>::Repr), (usize, usize))) -> ColumnIter<'a, P>,
>;
pub type ColumnsIterMut<'a, P> = Map<
    Zip<Enumerate<IterMut<'a, <P as Pixel>::Repr>>, Repeat<(usize, usize)>>,
    fn(((usize, &'a mut <P as Pixel>::Repr), (usize, usize))) -> ColumnIterMut<'a, P>,
>;
#[cfg(feature = "rayon")]
pub type ParColumnsIter<'a, P> = ::rayon::iter::Map<
    ::rayon::iter::Zip<
        ::rayon::iter::Enumerate<::rayon::slice::Iter<'a, <P as Pixel>::Repr>>,
        ::rayon::iter::RepeatN<(usize, usize)>,
    >,
    fn(((usize, &'a <P as Pixel>::Repr), (usize, usize))) -> ColumnIter<'a, P>,
>;
#[cfg(feature = "rayon")]
pub type ParColumnsIterMut<'a, P> = ::rayon::iter::Map<
    ::rayon::iter::Zip<
        ::rayon::iter::Enumerate<::rayon::slice::IterMut<'a, <P as Pixel>::Repr>>,
        ::rayon::iter::RepeatN<(usize, usize)>,
    >,
    fn(((usize, &'a mut <P as Pixel>::Repr), (usize, usize))) -> ColumnIterMut<'a, P>,
>;

/// Image operations
// # Safety
// - The dimensions of the image must be correct and consistent with the data length (width * height * channels)
// - The data must be in row-major order and contiguous
pub trait ImageOps: Sealed {
    type Scalar: Scalar;
    type PixelRepr: PixelRepr<Scalar = Self::Scalar>;
    type Pixel: Pixel<Scalar = Self::Scalar, Repr = Self::PixelRepr>;
    /// Pixel width of the image, must be consistent with the data
    fn width(&self) -> usize;
    /// Pixel height of the image, must be consistent with the data
    fn height(&self) -> usize;
    /// Dimensions of the image
    #[inline]
    fn dimensions(&self) -> (usize, usize) { (self.width(), self.height()) }
    /// Get a reference to the underlying raw data.
    fn data(&self) -> &[Self::Scalar];
    /// Get a reference to the flattened pixel data.
    #[inline]
    fn pixels(&self) -> &[Self::PixelRepr] { pixelate::<Self::Pixel>(self.data()) }
    /// Get the pixel at (x, y)
    fn get(&self, x: usize, y: usize) -> Option<&Self::PixelRepr> {
        if x < self.width() && y < self.height() {
            Some(unsafe { self.get_unchecked(x, y) })
        } else {
            None
        }
    }
    /// Get the pixel at (x, y) unchecked
    unsafe fn get_unchecked(&self, x: usize, y: usize) -> &Self::PixelRepr {
        self.pixels().get_unchecked(y * self.width() + x)
    }
    /// Clone `self` into a new [`Image`]
    #[inline]
    fn to_owned(&self) -> Image<Self::Pixel> {
        unsafe { Image::from_raw_unchecked(self.data().to_vec(), self.width(), self.height()) }
    }
    /// Create an iterator over the rows of this image, each row is a slice of pixels
    #[inline]
    fn rows(&self) -> ChunksExact<Self::PixelRepr> { self.pixels().chunks_exact(self.width()) }
    /// Create an iterator over the columns of this image, each column is a iterator over pixels
    fn columns(&self) -> ColumnsIter<Self::Pixel> {
        self.pixels()[..self.width()]
            .iter()
            .enumerate()
            .zip(::core::iter::repeat((self.pixels().len(), self.width())))
            .map(_map_column::<Self::Pixel>)
    }
    /// Create a parallel iterator over the rows of this image, each row is a slice of pixels
    #[cfg(feature = "rayon")]
    fn par_rows(&self) -> ::rayon::slice::ChunksExact<Self::PixelRepr> { self.pixels().par_chunks_exact(self.width()) }
    /// Create a parallel iterator over the columns of this image, each column is a iterator over pixels
    #[cfg(feature = "rayon")]
    fn par_columns(&self) -> ParColumnsIter<Self::Pixel> {
        self.pixels()[..self.width()]
            .par_iter()
            .enumerate()
            .zip(::rayon::iter::repeatn((self.pixels().len(), self.width()), self.width()))
            .map(_map_column::<Self::Pixel>)
    }
    /// Resize the image to the given dimensions, not preserving aspect ratio.
    fn resize(&self, width: usize, height: usize, filter: &FilterType) -> Image<Self::Pixel> {
        let (w, h) = self.dimensions();
        if width == w && height == h {
            return self.to_owned();
        }
        let mut buf = unsafe { Image::new_uninit_unchecked(width, height) };
        sample(filter.filter(), ImageView { width: w, height: h, data: self.data() }, buf.view_mut());
        buf
    }
    /// Resize within the given dimensions, preserving aspect ratio.
    ///
    /// For example with an image with dimensions (100, 100):
    ///  - `image.resize_within(100,50,_)` is the same as `image.resize(50,50,_)`.
    ///  - `image.resize_within(200,300,_)` is the same as `image.resize(200,200,_)`.
    fn rescale(&self, width: usize, height: usize, filter: &FilterType) -> Image<Self::Pixel> {
        let (width, height) = squeeze_dimensions(self.dimensions(), (width, height));
        self.resize(width, height, filter)
    }
    /// [`resize`](ImageOps::resize),
    /// except if the image is larger than the given dimensions * multiplicty,
    /// it will be downsampled by nearest neighbor first.
    ///
    /// This helps avoid unecessary work when sampling from a large image.
    fn supersize(&self, width: usize, height: usize, filter: &FilterType, multiplicity: f32) -> Image<Self::Pixel> {
        let (w, h) = self.dimensions();
        if width == w && height == h {
            return self.to_owned();
        }
        let mut buf = unsafe { Image::new_uninit_unchecked(width, height) };
        supersample(filter.filter(), ImageView { width: w, height: h, data: self.data() }, buf.view_mut(), multiplicity);
        buf
    }
    /// [`rescale`](ImageOps::rescale),
    /// except if the image is larger than the given dimensions * multiplicty,
    /// it will be downsampled by nearest neighbor first.
    ///
    /// This helps avoid unecessary work when sampling from a large image.
    fn superscale(&self, width: usize, height: usize, filter: &FilterType, multiplicity: f32) -> Image<Self::Pixel> {
        let (width, height) = squeeze_dimensions(self.dimensions(), (width, height));
        self.supersize(width, height, filter, multiplicity)
    }
    /// [`resize`](ImageOps::resize) using SIMD.
    ///
    /// Only available if the `fir` feature is enabled and the pixel type is compatible.
    #[cfg(feature = "fir")]
    fn fir_resize(&self, width: usize, height: usize, filter: &FilterType) -> Image<Self::Pixel>
    where
        Self::Scalar: CompatScalar,
        Self::PixelRepr: CompatPixelRepr,
        Self::Pixel: CompatPixel,
    {
        let (w, h) = self.dimensions();
        if width == w && height == h {
            return self.to_owned();
        }
        let mut buf = unsafe { Image::new_uninit_unchecked(width, height) };
        let mut resizer = ::fast_image_resize::Resizer::new(filter.algorithm());
        let v = <Self::Pixel as CompatPixel>::fir_view(ImageView { width: w, height: h, data: self.data() });
        let mut mv = <Self::Pixel as CompatPixel>::fir_view_mut(buf.view_mut());
        // The only error that can (should) occur is if memory is corrupted, which is a bug.
        resizer.resize(&v, &mut mv).expect(concat!("something went wrong: ", module_path!(), "::ImageOps::fir_resize"));
        buf
    }
    /// [`rescale`](ImageOps::rescale) using SIMD.
    ///
    /// Only available if the `fir` feature is enabled and the pixel type is compatible.
    #[cfg(feature = "fir")]
    fn fir_rescale(&self, width: usize, height: usize, filter: &FilterType) -> Image<Self::Pixel>
    where
        Self::Scalar: CompatScalar,
        Self::PixelRepr: CompatPixelRepr,
        Self::Pixel: CompatPixel,
    {
        let (width, height) = squeeze_dimensions(self.dimensions(), (width, height));
        self.fir_resize(width, height, filter)
    }
    /// [`supersize`](ImageOps::supersize) using SIMD.
    ///
    /// multiplicity here uses u8 instead of f32 due to the limitations of the underlying library.
    ///
    /// Only available if the `fir` feature is enabled and the pixel type is compatible.
    #[cfg(feature = "fir")]
    fn fir_supersize(&self, width: usize, height: usize, filter: &FilterType, multiplicity: u8) -> Image<Self::Pixel>
    where
        Self::Scalar: CompatScalar,
        Self::PixelRepr: CompatPixelRepr,
        Self::Pixel: CompatPixel,
    {
        let (w, h) = self.dimensions();
        if width == w && height == h {
            return self.to_owned();
        }
        let mut buf = unsafe { Image::new_uninit_unchecked(width, height) };
        let mut resizer = ::fast_image_resize::Resizer::new(filter.ss_algorithm(multiplicity));
        let v = <Self::Pixel as CompatPixel>::fir_view(ImageView { width: w, height: h, data: self.data() });
        let mut mv = <Self::Pixel as CompatPixel>::fir_view_mut(buf.view_mut());
        // The only error that can (should) occur is if memory is corrupted, which is a bug.
        resizer.resize(&v, &mut mv).expect(concat!("something went wrong: ", module_path!(), "::ImageOps::fir_supersize"));
        buf
    }
    /// [`superscale`](ImageOps::superscale) using SIMD.
    ///
    /// multiplicity here uses u8 instead of f32 due to the limitations of the underlying library.
    ///
    /// Only available if the `fir` feature is enabled and the pixel type is compatible.
    #[cfg(feature = "fir")]
    fn fir_superscale(&self, width: usize, height: usize, filter: &FilterType, multiplicity: u8) -> Image<Self::Pixel>
    where
        Self::Scalar: CompatScalar,
        Self::PixelRepr: CompatPixelRepr,
        Self::Pixel: CompatPixel,
    {
        let (width, height) = squeeze_dimensions(self.dimensions(), (width, height));
        self.fir_supersize(width, height, filter, multiplicity)
    }
}

/// Mutable image operations
// # Safety
// - The dimensions of the image must be correct and consistent with the data length (width * height * channels)
// - The data must be in row-major order and contiguous
pub trait ImageOpsMut: ImageOps {
    /// Get a mutable reference to the underlying raw data.
    fn data_mut(&mut self) -> &mut [Self::Scalar];
    /// Get a mutable reference to the flattened pixel data.
    #[inline]
    fn pixels_mut(&mut self) -> &mut [Self::PixelRepr] { pixelate_mut::<Self::Pixel>(self.data_mut()) }
    /// Get a mutable reference to the pixel at (x, y)
    fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Self::PixelRepr> {
        let (w, h) = self.dimensions();
        if x < w && y < h {
            Some(unsafe { self.pixels_mut().get_unchecked_mut(y * w + x) })
        } else {
            None
        }
    }
    /// Get a mutable reference to the pixel at (x, y) unchecked.
    unsafe fn get_unchecked_mut(&mut self, x: usize, y: usize) -> &mut Self::PixelRepr {
        let w = self.width();
        self.pixels_mut().get_unchecked_mut(y * w + x)
    }
    /// Create an iterator over the mutable rows of the image.
    #[inline]
    fn rows_mut(&mut self) -> ChunksExactMut<Self::PixelRepr> {
        let w = self.width();
        self.pixels_mut().chunks_exact_mut(w)
    }
    /// Create an iterator over the mutable columns of the image.
    fn columns_mut(&mut self) -> ColumnsIterMut<Self::Pixel> {
        let w = self.width();
        let pxs = self.pixels_mut();
        let pxs_len = pxs.len();
        pxs[..w].iter_mut().enumerate().zip(::core::iter::repeat((pxs_len, w))).map(_map_column_mut::<Self::Pixel>)
    }
    /// Create a parallel iterator over the mutable rows of the image.
    #[inline]
    #[cfg(feature = "rayon")]
    fn par_rows_mut(&mut self) -> ::rayon::slice::ChunksExactMut<Self::PixelRepr> {
        let w = self.width();
        self.pixels_mut().par_chunks_exact_mut(w)
    }
    /// Create a parallel iterator over the mutable columns of the image.
    #[cfg(feature = "rayon")]
    fn par_columns_mut(&mut self) -> ParColumnsIterMut<Self::Pixel> {
        let w = self.width();
        let pxs = self.pixels_mut();
        let pxs_len = pxs.len();
        pxs[..w].par_iter_mut().enumerate().zip(::rayon::iter::repeatn((pxs_len, w), w)).map(_map_column_mut::<Self::Pixel>)
    }
}

impl<P: Pixel> Sealed for Image<P> {}
impl<P: Pixel> Sealed for ImageView<'_, P> {}
impl<P: Pixel> Sealed for ImageViewMut<'_, P> {}

impl<P: Pixel> ImageOps for Image<P> {
    type Scalar = <P::Repr as PixelRepr>::Scalar;
    type PixelRepr = P::Repr;
    type Pixel = P;
    #[inline(always)]
    fn width(&self) -> usize { self.width }
    #[inline(always)]
    fn height(&self) -> usize { self.height }
    #[inline(always)]
    fn data(&self) -> &[Self::Scalar] { self.data.as_ref() }
}
impl<P: Pixel> ImageOpsMut for Image<P> {
    #[inline(always)]
    fn data_mut(&mut self) -> &mut [Self::Scalar] { self.data.as_mut() }
}
impl<'a, P: Pixel> ImageOps for ImageView<'a, P> {
    type Scalar = <P::Repr as PixelRepr>::Scalar;
    type PixelRepr = P::Repr;
    type Pixel = P;
    #[inline(always)]
    fn width(&self) -> usize { self.width }
    #[inline(always)]
    fn height(&self) -> usize { self.height }
    #[inline(always)]
    fn data(&self) -> &[Self::Scalar] { self.data }
}
impl<'a, P: Pixel> ImageOps for ImageViewMut<'a, P> {
    type Scalar = <P::Repr as PixelRepr>::Scalar;
    type PixelRepr = P::Repr;
    type Pixel = P;
    #[inline(always)]
    fn width(&self) -> usize { self.width }
    #[inline(always)]
    fn height(&self) -> usize { self.height }
    #[inline(always)]
    fn data(&self) -> &[Self::Scalar] { self.data }
}
impl<'a, P: Pixel> ImageOpsMut for ImageViewMut<'a, P> {
    #[inline(always)]
    fn data_mut(&mut self) -> &mut [Self::Scalar] { self.data }
}

macro_rules! impl_Index {
    () => {
        type Output = <Self as ImageOps>::PixelRepr;
        #[inline(always)]
        fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
            assert!(x < self.width() && y < self.height());
            unsafe { self.pixels().get_unchecked(y * self.width() + x) }
        }
    };
}
macro_rules! impl_IndexMut {
    () => {
        #[inline(always)]
        fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
            assert!(x < self.width() && y < self.height());
            let i = y * self.width() + x;
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
impl<'a, P: Pixel> From<&'a Image<P>> for ImageView<'a, P> {
    #[inline(always)]
    fn from(value: &'a Image<P>) -> Self { ImageView { width: value.width, height: value.height, data: &value.data } }
}
impl<'a, P: Pixel> From<&'a mut Image<P>> for ImageViewMut<'a, P> {
    #[inline(always)]
    fn from(value: &'a mut Image<P>) -> Self {
        ImageViewMut { width: value.width, height: value.height, data: &mut value.data }
    }
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

/// Fill the given dimensions with the given image dimensions, keeping the aspect ratio.
pub fn fill_dimensions(src: (usize, usize), dst: (usize, usize)) -> (usize, usize) {
    let (w, h) = src;
    let (nw, nh) = dst;
    let ratio = f64::max(nw as f64 / w as f64, nh as f64 / h as f64);
    let (w, h) = (w as f64 * ratio, h as f64 * ratio);
    (w as usize, h as usize)
}
