use std::marker::PhantomData;

use viuwa_image::*;

use super::*;

/// Wrapper around any [`Image`], [`ImageView`], or [`ImageViewMut`] with a [`Pixel`] that implements [`AnsiPixel`]
/// to provide methods for converting the pixels to ANSI escape sequences on iteration, requires that character is a [`CharBytes`]
pub struct AnsiImageIter<Image: ImageOps, C: Converter>
where
    Image::Pixel: AnsiPixel,
{
    pub image: Image,
    pub char: CharBytes,
    pub attributes: ColorAttributes,
    _phantom: PhantomData<C>,
}

/// Wrapper around any [`Image`], [`ImageView`], or [`ImageViewMut`] with a [`Pixel`] that implements [`AnsiPixel`]
/// to provide methods for converting the pixels to ANSI escape sequences
pub struct AnsiImage<Image: ImageOps, C: Converter, Char: Bytes>
where
    Image::Pixel: AnsiPixel,
{
    pub image: Image,
    pub char: Char,
    pub attributes: ColorAttributes,
    _phantom: PhantomData<C>,
}

impl<Image: ImageOps, C: Converter, Char: Bytes> AnsiImage<Image, C, Char>
where
    Image::Pixel: AnsiPixel,
{
    pub fn new(image: Image, char: CharBytes, attributes: ColorAttributes) -> Self {
        Self { image, char, attributes, _phantom: PhantomData }
    }
    pub fn width(&self) -> usize { self.image.width() }
    pub fn height(&self) -> usize { (self.image.height() as f32 / 2.).ceil() as usize }
    pub fn rows<'a>(&'a self) -> ::core::iter::Map<_, _> {
        self.image.rows().zip(::core::iter::repeat((self.char, self.attributes))).map(mapper_half(_))
    }
}

mod iter {
    use ::core::{iter::*, slice::*};

    use super::*;

    pub(crate) fn iter_mapper_full<P: AnsiPixel, C: Converter>(
        ((fg, bg), (c, a)): ((P::Repr, P::Repr), (CharBytes, ColorAttributes)),
    ) -> <C::Sequencer as Sequencer>::FullChar {
        // Safe because any `Converter` is Sealed and we guarantee ourselves that the size is correct
        unsafe { *(&mut (C::full::<P>(fg, bg, a), c) as *mut _ as *mut _) }
    }
    pub(crate) fn iter_mapper_half<P: AnsiPixel, C: Converter>(
        (fg, (c, a)): (P::Repr, (CharBytes, ColorAttributes)),
    ) -> <C::Sequencer as Sequencer>::HalfChar {
        // Safe because any `Converter` is Sealed and we guarantee ourselves that the size is correct
        unsafe { *(&mut (C::fg::<P>(fg, a), c) as *mut _ as *mut _) }
    }
    pub(crate) fn mapper_half<'a, P: AnsiPixel, C: Converter, Char: Bytes>(
        (fgs, (c, a)): (&'a [P::Repr], (Char, ColorAttributes)),
    ) -> Box<[u8]> {
        ::bytemuck::cast_slice_box(fgs.into_iter().copied().map(|fg| (C::fg::<P>(fg, a), c)).collect())
    }

    pub struct AnsiRowsIter<'a, P: AnsiPixel, C: Converter>(
        pub(crate) ChunksExact<'a, P::Repr>,
        pub(crate) CharBytes,
        pub(crate) ColorAttributes,
        pub(crate) PhantomData<C>,
    );
    pub enum AnsiRowIter<'a, P: AnsiPixel, C: Converter> {
        /// A row of ANSI that fills the height of a cell
        Full(
            Map<
                Zip<Zip<Copied<Iter<'a, P::Repr>>, Copied<Iter<'a, P::Repr>>>, Repeat<(CharBytes, ColorAttributes)>>,
                fn(((P::Repr, P::Repr), (CharBytes, ColorAttributes))) -> <C::Sequencer as Sequencer>::FullChar,
            >,
        ),
        /// A row of ANSI that only fills the top half of a cell
        Half(
            Map<
                Zip<Copied<Iter<'a, P::Repr>>, Repeat<(CharBytes, ColorAttributes)>>,
                fn((P::Repr, (CharBytes, ColorAttributes))) -> <C::Sequencer as Sequencer>::HalfChar,
            >,
        ),
    }
    impl<'a, P: AnsiPixel, C: Converter> Iterator for AnsiRowsIter<'a, P, C> {
        type Item = AnsiRowIter<'a, P, C>;
        fn next(&mut self) -> Option<Self::Item> {
            use ::core::iter::*;
            match (self.0.next(), self.0.next()) {
                (Some(fgs), Some(bgs)) => Some(AnsiRowIter::Full(
                    fgs.iter().copied().zip(bgs.iter().copied()).zip(repeat((self.1, self.2))).map(iter_mapper_full::<P, C>),
                )),
                (Some(fgs), None) => {
                    Some(AnsiRowIter::Half(fgs.iter().copied().zip(repeat((self.1, self.2))).map(iter_mapper_half::<P, C>)))
                }
                _ => None,
            }
        }
    }
}
pub use iter::*;

impl<Image: ImageOps, C: Converter> AnsiImageIter<Image, C>
where
    Image::Pixel: AnsiPixel,
{
    /// Create a new AnsiImage from a [`Image`], [`ImageView`], or [`ImageViewMut`]
    pub fn new(image: Image, char_bytes: CharBytes, attributes: ColorAttributes) -> Self {
        Self { image, char: char_bytes, attributes, _phantom: PhantomData }
    }
    /// The width of the image in characters
    pub fn width(&self) -> usize { self.image.width() }
    /// The height of the image in characters
    pub fn height(&self) -> usize { (self.image.height() as f32 / 2.).ceil() as usize }
    /// Create an iterator over the character rows of the image
    pub fn rows<'a>(&'a self) -> AnsiRowsIter<'a, Image::Pixel, C> {
        AnsiRowsIter(self.image.rows(), self.char, self.attributes, PhantomData)
    }
    /// Create an iterator over the character rows of the image, using [`rayon`] to parallelize the process
    #[cfg(feature = "rayon")]
    pub fn par_rows<'a>(&'a self) -> impl ParallelIterator<Item = AnsiRowIter<'a, Image::Pixel, C>> + 'a {
        use ::rayon::iter::*;
        fn mapper<P: AnsiPixel, C: Converter>(
            (r, (c, a)): (Vec<&[P::Repr]>, (CharBytes, ColorAttributes)),
        ) -> AnsiRowIter<P, C> {
            match r.as_slice() {
                [fgs, bgs] => AnsiRowIter::Full(
                    fgs.iter()
                        .copied()
                        .zip(bgs.iter().copied())
                        .zip(::core::iter::repeat((c, a)))
                        .map(iter_mapper_full::<P, C>),
                ),
                [fgs] => {
                    AnsiRowIter::Half(fgs.iter().copied().zip(::core::iter::repeat((c, a))).map(iter_mapper_half::<P, C>))
                }
                _ => unreachable!(),
            }
        }
        self.image
            .par_rows()
            .chunks(2)
            .zip(repeat((self.char, self.attributes)).take(self.height()))
            .map(mapper::<Image::Pixel, C>)
    }
}

/// Wrapper around any [`Image`], [`ImageView`], or [`ImageViewMut`] with a [`Pixel`] that implements [`AnsiPixel`]
/// to provide methods for converting the pixels to ANSI escape sequences
pub enum DynamicAnsiImage<Image: ImageOps>
where
    Image::Pixel: AnsiPixel,
{
    Color(AnsiImageIter<Image, ColorConverter>),
    Gray(AnsiImageIter<Image, GrayConverter>),
    AnsiColor(AnsiImageIter<Image, AnsiColorConverter>),
    AnsiGray(AnsiImageIter<Image, AnsiGrayConverter>),
}
impl<Image: ImageOps> From<AnsiImageIter<Image, ColorConverter>> for DynamicAnsiImage<Image>
where
    Image::Pixel: AnsiPixel,
{
    fn from(image: AnsiImageIter<Image, ColorConverter>) -> Self { Self::Color(image) }
}
impl<Image: ImageOps> From<AnsiImageIter<Image, GrayConverter>> for DynamicAnsiImage<Image>
where
    Image::Pixel: AnsiPixel,
{
    fn from(image: AnsiImageIter<Image, GrayConverter>) -> Self { Self::Gray(image) }
}
impl<Image: ImageOps> From<AnsiImageIter<Image, AnsiColorConverter>> for DynamicAnsiImage<Image>
where
    Image::Pixel: AnsiPixel,
{
    fn from(image: AnsiImageIter<Image, AnsiColorConverter>) -> Self { Self::AnsiColor(image) }
}
impl<Image: ImageOps> From<AnsiImageIter<Image, AnsiGrayConverter>> for DynamicAnsiImage<Image>
where
    Image::Pixel: AnsiPixel,
{
    fn from(image: AnsiImageIter<Image, AnsiGrayConverter>) -> Self { Self::AnsiGray(image) }
}
