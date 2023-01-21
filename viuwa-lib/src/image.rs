use ::core::{
    iter::*,
    slice::{ChunksExact, ChunksExactMut, Iter, IterMut},
};
use ::image::Pixel;

use super::*;

/// Any type that can be used as container for flat image pixel data within this library.
/// (e.g. Vec, Box, [u8; 3], etc.)
///
/// Currently only implemented for types that implement `Into<Box<[Scalar]>>`
pub trait Container<P: RawPixel>:
    Clone + Into<Box<[<P::Repr as PixelRepr>::Scalar]>> + ::core::ops::Deref<Target = [<P::Repr as PixelRepr>::Scalar]>
{
}
impl<
        P: RawPixel,
        C: Clone + Into<Box<[<P::Repr as PixelRepr>::Scalar]>> + ::core::ops::Deref<Target = [<P::Repr as PixelRepr>::Scalar]>,
    > Container<P> for C
{
}

/// Image API with a pixel type `P`, used for internal image representation.
#[derive(Clone, Default)]
pub struct Image<P: RawPixel> {
    /// The image data
    pub(crate) data: Box<[<P::Repr as PixelRepr>::Scalar]>,
    /// The pixel width of the image
    pub(crate) width: usize,
    /// The pixel height of the image
    pub(crate) height: usize,
}
/// Explicitly immutable image view for use with unowned data
#[derive(Clone, Copy)]
pub struct ImageView<'a, P: RawPixel> {
    /// The image data
    pub(crate) data: &'a [<P::Repr as PixelRepr>::Scalar],
    /// The pixel width of the image
    pub(crate) width: usize,
    /// The pixel height of the image
    pub(crate) height: usize,
}
/// Explicitly mutable image view for use with unowned data
pub struct ImageViewMut<'a, P: RawPixel> {
    /// The image data
    pub(crate) data: &'a mut [<P::Repr as PixelRepr>::Scalar],
    /// The pixel width of the image
    pub(crate) width: usize,
    /// The pixel height of the image
    pub(crate) height: usize,
}

impl<P: RawPixel> Image<P> {
    /// Create a new image with zeroed data
    /// # Errors
    /// If the image size is too large to fit into usize
    pub fn new(width: u32, height: u32) -> Result<Self, ()> {
        let len = (width as usize).checked_mul(height as usize).ok_or(())?.checked_mul(P::Repr::CHANNELS).ok_or(())?;
        Ok(Self {
            data: vec![<<P::Repr as PixelRepr>::Scalar as ::num_traits::Zero>::zero(); len].into(),
            width: width as usize,
            height: height as usize,
        })
    }
    /// Explicit unitialized constructor, prefer to use `new` instead.
    pub unsafe fn new_uninit(width: usize, height: usize) -> Self {
        Self { data: vec![uninit!(); width * height * P::Repr::CHANNELS].into(), width, height }
    }
    /// Create a new image with the given data
    /// # Errors
    /// If the data is not of the correct length (width * height * channels)
    pub fn from_raw<C: Container<P>>(data: C, width: usize, height: usize) -> Result<Self, C> {
        if data.len() != usize::MAX && data.len() == width.saturating_mul(height).saturating_mul(P::Repr::CHANNELS) {
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
}
impl<'a, P: RawPixel> ImageView<'a, P> {
    /// Create a new image view with the given data
    pub fn new(image: &'a Image<P>) -> Self { Self { data: image.data.as_ref(), width: image.width, height: image.height } }
    /// Create a new image view with the given data
    /// # Errors
    /// If the data is not of the correct length (width * height * channels)
    pub fn from_raw(
        data: &'a [<P::Repr as PixelRepr>::Scalar],
        width: usize,
        height: usize,
    ) -> Result<Self, &'a [<P::Repr as PixelRepr>::Scalar]> {
        if data.len() != usize::MAX && data.len() == width.saturating_mul(height).saturating_mul(P::Repr::CHANNELS) {
            Ok(Self { data, width, height })
        } else {
            Err(data)
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
impl<'a, P: RawPixel> ImageViewMut<'a, P> {
    /// Create a new image view with the given data
    pub fn new(image: &'a mut Image<P>) -> Self {
        Self { data: image.data.as_mut(), width: image.width, height: image.height }
    }
    /// Create a new image view with the given data
    /// # Errors
    /// If the data is not of the correct length (width * height * channels)
    pub fn from_raw(
        data: &'a mut [<P::Repr as PixelRepr>::Scalar],
        width: usize,
        height: usize,
    ) -> Result<Self, &'a mut [<P::Repr as PixelRepr>::Scalar]> {
        if data.len() != usize::MAX && data.len() == width.saturating_mul(height).saturating_mul(P::Repr::CHANNELS) {
            Ok(Self { data, width, height })
        } else {
            Err(data)
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

/// Cast a slice of scalars to a slice of pixel representations
#[inline]
pub(crate) fn _pixelate<'a, P: RawPixel>(data: &'a [<P::Repr as PixelRepr>::Scalar]) -> &'a [P::Repr] {
    unsafe { ::core::slice::from_raw_parts(data.as_ptr().cast(), data.len() / P::Repr::CHANNELS) }
}
/// Cast a slice of scalars to a slice of pixel representations unchecked
#[inline]
pub(crate) unsafe fn _pixelate_unchecked<'a, P: RawPixel>(
    data: &'a [<P::Repr as PixelRepr>::Scalar],
    len: usize,
) -> &'a [P::Repr] {
    unsafe { ::core::slice::from_raw_parts(data.as_ptr().cast(), len) }
}
/// Cast a slice of scalars to a slice of pixel representations
#[inline]
pub(crate) fn _pixelate_mut<'a, P: RawPixel>(data: &'a mut [<P::Repr as PixelRepr>::Scalar]) -> &'a mut [P::Repr] {
    unsafe { ::core::slice::from_raw_parts_mut(data.as_mut_ptr().cast(), data.len() / P::Repr::CHANNELS) }
}
/// Cast a slice of scalars to a slice of pixel representations unchecked
#[inline]
pub(crate) unsafe fn _pixelate_unchecked_mut<'a, P: RawPixel>(
    data: &'a mut [<P::Repr as PixelRepr>::Scalar],
    len: usize,
) -> &'a mut [P::Repr] {
    unsafe { ::core::slice::from_raw_parts_mut(data.as_mut_ptr().cast(), len) }
}

fn _map_column<P: RawPixel>(((skip, start), (len, step)): ((usize, &P::Repr), (usize, usize))) -> StepBy<Iter<P::Repr>> {
    unsafe { ::core::slice::from_raw_parts(start as *const P::Repr, len - skip) }.iter().step_by(step)
}
type ColumnMapper<P: RawPixel> = fn(((usize, &P::Repr), (usize, usize))) -> StepBy<Iter<P::Repr>>;
fn _map_column_mut<P: RawPixel>(
    ((skip, start), (len, step)): ((usize, &mut P::Repr), (usize, usize)),
) -> StepBy<IterMut<P::Repr>> {
    unsafe { ::core::slice::from_raw_parts_mut(start as *mut P::Repr, len - skip) }.iter_mut().step_by(step)
}
type ColumnMapperMut<P: RawPixel> = fn(((usize, &mut P::Repr), (usize, usize))) -> StepBy<IterMut<P::Repr>>;

/// A trait for images that are backed by a flat array of scalars in row-major order
/// # Safety
/// - The dimensions of the image must be correct and consistent with the data length (width * height * channels)
/// - The data must be in row-major order and contiguous
pub unsafe trait ImageOps {
    type Scalar: Scalar;
    type PixelRepr: PixelRepr<Scalar = Self::Scalar>;
    type Pixel: RawPixel<Repr = Self::PixelRepr>;
    /// Pixel width of the image, must be consistent with the data
    fn width(&self) -> usize;
    /// Pixel height of the image, must be consistent with the data
    fn height(&self) -> usize;
    /// Dimensions of the image
    #[inline]
    fn dimensions(&self) -> (usize, usize) { (self.width(), self.height()) }
    /// Flattened image data
    fn data(&self) -> &[Self::Scalar];
    /// Image data as a slice of pixels
    #[inline]
    fn pixels(&self) -> &[Self::PixelRepr] {
        unsafe { _pixelate_unchecked::<Self::Pixel>(self.data(), self.width() * self.height()) }
    }
    /// Pixel row iterator
    #[inline]
    fn rows(&self) -> ChunksExact<Self::PixelRepr> { self.pixels().chunks_exact(self.width()) }
    /// Pixel column iterator
    fn columns(&self) -> Map<Zip<Enumerate<Iter<Self::PixelRepr>>, Repeat<(usize, usize)>>, ColumnMapper<Self::Pixel>> {
        self.pixels()[..self.width()]
            .iter()
            .enumerate()
            .zip(::core::iter::repeat((self.pixels().len(), self.width())))
            .map(_map_column::<Self::Pixel>)
    }
    #[cfg(feature = "rayon")]
    fn par_rows(&self) -> ::rayon::slice::ChunksExact<Self::PixelRepr> { self.pixels().par_chunks_exact(self.width()) }
    #[cfg(feature = "rayon")]
    fn par_columns(
        &self,
    ) -> ::rayon::iter::Map<
        ::rayon::iter::Zip<
            ::rayon::iter::Enumerate<::rayon::slice::Iter<Self::PixelRepr>>,
            ::rayon::iter::RepeatN<(usize, usize)>,
        >,
        ColumnMapper<Self::Pixel>,
    > {
        self.pixels()[..self.width()]
            .par_iter()
            .enumerate()
            .zip(::rayon::iter::repeatn((self.pixels().len(), self.width()), self.width()))
            .map(_map_column::<Self::Pixel>)
    }
}

/// A trait for images that are backed by a flat array of scalars in row-major order
/// # Safety
/// - The dimensions of the image must be correct and consistent with the data length (width * height * channels)
/// - The data must be in row-major order and contiguous
pub unsafe trait ImageOpsMut: ImageOps {
    fn data_mut(&mut self) -> &mut [Self::Scalar];
    #[inline]
    fn pixels_mut(&mut self) -> &mut [Self::PixelRepr] {
        let len = self.width() * self.height();
        unsafe { _pixelate_unchecked_mut::<Self::Pixel>(self.data_mut(), len) }
    }
    #[inline]
    fn rows_mut(&mut self) -> ChunksExactMut<Self::PixelRepr> {
        let w = self.width();
        self.pixels_mut().chunks_exact_mut(w)
    }
    fn columns_mut(
        &mut self,
    ) -> Map<Zip<Enumerate<IterMut<Self::PixelRepr>>, Repeat<(usize, usize)>>, ColumnMapperMut<Self::Pixel>> {
        let w = self.width();
        let pxs = self.pixels_mut();
        let pxs_len = pxs.len();
        pxs[..w].iter_mut().enumerate().zip(::core::iter::repeat((pxs_len, w))).map(_map_column_mut::<Self::Pixel>)
    }
    #[inline]
    #[cfg(feature = "rayon")]
    fn par_rows_mut(&mut self) -> ::rayon::slice::ChunksExactMut<Self::PixelRepr> {
        let w = self.width();
        self.pixels_mut().par_chunks_exact_mut(w)
    }
    #[cfg(feature = "rayon")]
    fn par_columns_mut(
        &mut self,
    ) -> ::rayon::iter::Map<
        ::rayon::iter::Zip<
            ::rayon::iter::Enumerate<::rayon::slice::IterMut<Self::PixelRepr>>,
            ::rayon::iter::RepeatN<(usize, usize)>,
        >,
        ColumnMapperMut<Self::Pixel>,
    > {
        let w = self.width();
        let pxs = self.pixels_mut();
        let pxs_len = pxs.len();
        pxs[..w].par_iter_mut().enumerate().zip(::rayon::iter::repeatn((pxs_len, w), w)).map(_map_column_mut::<Self::Pixel>)
    }
}

unsafe impl<P: RawPixel> ImageOps for Image<P> {
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
unsafe impl<P: RawPixel> ImageOpsMut for Image<P> {
    #[inline(always)]
    fn data_mut(&mut self) -> &mut [Self::Scalar] { self.data.as_mut() }
}
unsafe impl<'a, P: RawPixel> ImageOps for ImageView<'a, P> {
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
unsafe impl<'a, P: RawPixel> ImageOps for ImageViewMut<'a, P> {
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
unsafe impl<'a, P: RawPixel> ImageOpsMut for ImageViewMut<'a, P> {
    #[inline(always)]
    fn data_mut(&mut self) -> &mut [Self::Scalar] { self.data }
}

// #[cfg(feature = "image")]
impl<
        T: Scalar,
        R: PixelRepr<Scalar = T>,
        P: ::image::Pixel<Subpixel = T> + RawPixel<Repr = R>,
        C: Clone + ::core::ops::Deref<Target = [T]> + From<Box<[T]>>,
    > From<Image<P>> for ::image::ImageBuffer<P, C>
{
    #[inline(always)]
    fn from(Image { data, width, height, .. }: Image<P>) -> Self {
        Self::from_raw(width as u32, height as u32, From::from(data)).unwrap()
    }
}
// #[cfg(feature = "image")]
impl<
        T: Scalar,
        R: PixelRepr<Scalar = T>,
        P: ::image::Pixel<Subpixel = T> + RawPixel<Repr = R>,
        C: Clone + ::core::ops::Deref<Target = [T]> + Into<Box<[T]>>,
    > From<::image::ImageBuffer<P, C>> for Image<P>
{
    #[inline(always)]
    fn from(image: ::image::ImageBuffer<P, C>) -> Self {
        let (w, h) = image.dimensions();
        Self { data: image.into_raw().into(), width: w as usize, height: h as usize }
    }
}
// #[cfg(feature = "image")]
impl<
        'a,
        T: Scalar + ::image::Primitive,
        R: PixelRepr<Scalar = T>,
        P: ::image::Pixel<Subpixel = T> + RawPixel<Repr = R>,
        C: Clone + ::core::ops::Deref<Target = [T]>,
    > From<&'a ImageBuffer<P, C>> for ImageView<'a, P>
{
    #[inline(always)]
    fn from(image: &'a ImageBuffer<P, C>) -> Self {
        let (w, h) = image.dimensions();
        Self { data: image.as_ref(), width: w as usize, height: h as usize }
    }
}
// #[cfg(feature = "image")]
impl<
        'a,
        T: Scalar + ::image::Primitive,
        R: PixelRepr<Scalar = T>,
        P: ::image::Pixel<Subpixel = T> + RawPixel<Repr = R>,
        C: Clone + ::core::ops::Deref<Target = [T]> + ::core::ops::DerefMut,
    > From<&'a mut ImageBuffer<P, C>> for ImageViewMut<'a, P>
{
    #[inline(always)]
    fn from(image: &'a mut ImageBuffer<P, C>) -> Self {
        let (w, h) = image.dimensions();
        Self { data: image.as_mut(), width: w as usize, height: h as usize }
    }
}
