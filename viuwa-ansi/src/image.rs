//! ## [`Order`]
//! [`Upper`] assigns foreground color to the upper half of the cell, e.g. `'▀'` <br>
//! [`Lower`] assigns foreground color to the lower half of the cell, e.g. `'▄'`
//!
//! NOTE: byte casting any tuple of Converters with the bytes of a Char is safe because they both are (*should be*) align 1.

use std::marker::PhantomData;

use viuwa_image::*;

use super::*;

/// Wrapper around any [`Image`], [`ImageView`], or [`ImageViewMut`] with a [`Pixel`] that implements [`AnsiPixel`]
/// to provide methods for converting the image to ANSI escape sequences by iteration.
///
/// It is strongly recommended to use a [`BufWriter`] to write to any unbuffered writer from the iterators,
/// an image in ANSI escape sequences can be *very* large (up to a factor of 40x the original image size).
pub struct AnsiImage<I: ImageOps, C: Converter>(I, PhantomData<C>)
where
    I::Pixel: AnsiPixel;

/// Wrapper around any [`Image`], [`ImageView`], or [`ImageViewMut`] with a [`Pixel`] that implements [`AnsiPixel`]
/// to provide methods for converting the pixels to ANSI escape sequences
pub enum DynamicAnsiImage<I: ImageOps>
where
    I::Pixel: AnsiPixel,
{
    Color(AnsiImage<I, ColorConverter>),
    Gray(AnsiImage<I, GrayConverter>),
    AnsiColor(AnsiImage<I, AnsiColorConverter>),
    AnsiGray(AnsiImage<I, AnsiGrayConverter>),
}

impl<I: ImageOps, C: Converter> AnsiImage<I, C>
where
    I::Pixel: AnsiPixel,
{
    /// Creates a new [`AnsiImage`] from a given [`Image`], [`ImageView`], or [`ImageViewMut`]
    pub fn new(image: I) -> Self { Self(image, PhantomData) }
    /// The width of the image in characters
    pub fn width(&self) -> usize { self.0.width() }
    /// The height of the image in characters
    pub fn height(&self) -> usize { div_ceil2(self.0.height()) }
    /// The dimensions of the image in characters
    pub fn dimensions(&self) -> (usize, usize) { (self.width(), self.height()) }
    /// Character rows iterator with a given [`char`] and [`ColorAttributes`]. <br>
    /// Where char is a character that mainly fills the upper half of the cell <br><br>
    /// `'▀'` will be used if `char` is `None`
    pub fn rows_upper(&mut self, attrs: ColorAttributes, char: Option<Char>) -> AnsiRows<I::Pixel, C, Upper> {
        AnsiRows { iter: self.0.rows(), char: char.unwrap_or(UPPER_HALF_BLOCK), attrs, phantom: PhantomData }
    }
    /// Character rows iterator with a given [`char`] and [`ColorAttributes`]. <br>
    /// Where char is a character that mainly fills the lower half of the cell <br><br>
    /// `'▄'` will be used if `char` is `None`
    pub fn rows_lower(&mut self, attrs: ColorAttributes, char: Option<Char>) -> AnsiRows<I::Pixel, C, Lower> {
        AnsiRows { iter: self.0.rows(), char: char.unwrap_or(LOWER_HALF_BLOCK), attrs, phantom: PhantomData }
    }
    #[cfg(feature = "rayon")]
    /// Parallel character rows iterator with a given [`char`] and [`ColorAttributes`].
    /// Where char is a character that mainly fills the upper half of the cell, e.g. `'▀'` <br>
    pub fn par_rows_upper(
        &mut self,
        attrs: ColorAttributes,
        char: Option<Char>,
    ) -> impl ParallelIterator<Item = AnsiRow<I::Pixel, C, Upper>> + IndexedParallelIterator
    where
        C: Send,
    {
        use rayon::iter::repeatn;
        let h = self.height();
        self.0.par_rows().chunks(2).zip(repeatn((char.unwrap_or(UPPER_HALF_BLOCK), attrs), h)).map(
            |(iter, (char, attrs))| match iter.as_slice() {
                [a, b] => AnsiRow::Full(FullAnsiRow { iter: ::core::iter::zip(*a, *b), char, attrs, phantom: PhantomData }),
                [a] => AnsiRow::Half(HalfAnsiRow { iter: a.iter(), char, attrs, phantom: PhantomData }),
                _ => unreachable!(),
            },
        )
    }
    /// Parallel character rows iterator with a given [`char`] and [`ColorAttributes`].
    /// Where char is a character that mainly fills the lower half of the cell, e.g. `'▄'` <br>
    #[cfg(feature = "rayon")]
    pub fn par_rows_lower(
        &mut self,
        attrs: ColorAttributes,
        char: Option<Char>,
    ) -> impl ParallelIterator<Item = AnsiRow<I::Pixel, C, Lower>> + IndexedParallelIterator
    where
        C: Send,
    {
        use rayon::iter::repeatn;
        let h = self.height();
        self.0.par_rows().chunks(2).zip(repeatn((char.unwrap_or(LOWER_HALF_BLOCK), attrs), h)).map(
            |(iter, (char, attrs))| match iter.as_slice() {
                [a, b] => AnsiRow::Full(FullAnsiRow { iter: ::core::iter::zip(*a, *b), char, attrs, phantom: PhantomData }),
                [a] => AnsiRow::Half(HalfAnsiRow { iter: a.iter(), char, attrs, phantom: PhantomData }),
                _ => unreachable!(),
            },
        )
    }
}

macro_rules! dyn_map {
    ($dyn:expr, $e:pat => $f:expr) => {{
        use Self::*;
        match $dyn {
            Color($e) => Color($f),
            Gray($e) => Gray($f),
            AnsiColor($e) => AnsiColor($f),
            AnsiGray($e) => AnsiGray($f),
        }
    }};
    ($dyn:expr, |$e: ident| $f:expr) => {
        match $dyn {
            Self::Color($e) => $f,
            Self::Gray($e) => $f,
            Self::AnsiColor($e) => $f,
            Self::AnsiGray($e) => $f,
        }
    };
}
impl<I: ImageOps> DynamicAnsiImage<I>
where
    I::Pixel: AnsiPixel,
{
    /// Creates a new [`DynamicAnsiImage`] from a given [`Image`], [`ImageView`], or [`ImageViewMut`] and a [`ColorType`]
    pub fn new(image: I, color: ColorType) -> Self {
        match color {
            ColorType::Color => Self::Color(AnsiImage::new(image)),
            ColorType::Gray => Self::Gray(AnsiImage::new(image)),
            ColorType::AnsiColor => Self::AnsiColor(AnsiImage::new(image)),
            ColorType::AnsiGray => Self::AnsiGray(AnsiImage::new(image)),
        }
    }
    /// The width of the image in characters
    pub fn width(&self) -> usize { dyn_map!(self, |image| image.width()) }
    /// The height of the image in characters
    pub fn height(&self) -> usize { dyn_map!(self, |image| image.height()) }
    /// The dimensions of the image in characters
    pub fn dimensions(&self) -> (usize, usize) { dyn_map!(self, |image| image.dimensions()) }
}

impl<I: ImageOps> From<AnsiImage<I, ColorConverter>> for DynamicAnsiImage<I>
where
    I::Pixel: AnsiPixel,
{
    #[inline(always)]
    fn from(image: AnsiImage<I, ColorConverter>) -> Self { Self::Color(image) }
}
impl<I: ImageOps> From<AnsiImage<I, GrayConverter>> for DynamicAnsiImage<I>
where
    I::Pixel: AnsiPixel,
{
    #[inline(always)]
    fn from(image: AnsiImage<I, GrayConverter>) -> Self { Self::Gray(image) }
}
impl<I: ImageOps> From<AnsiImage<I, AnsiColorConverter>> for DynamicAnsiImage<I>
where
    I::Pixel: AnsiPixel,
{
    #[inline(always)]
    fn from(image: AnsiImage<I, AnsiColorConverter>) -> Self { Self::AnsiColor(image) }
}
impl<I: ImageOps> From<AnsiImage<I, AnsiGrayConverter>> for DynamicAnsiImage<I>
where
    I::Pixel: AnsiPixel,
{
    #[inline(always)]
    fn from(image: AnsiImage<I, AnsiGrayConverter>) -> Self { Self::AnsiGray(image) }
}

#[inline(always)]
fn div_ceil2(n: usize) -> usize { (n >> 1) + (n & 1) }

mod iter {
    use ::core::{iter::*, slice::*};

    use super::*;

    #[inline(always)]
    fn fullchar<P: AnsiPixel, C: Converter>(
        fg: &<P>::Repr,
        bg: &<P>::Repr,
        c: Char,
        a: ColorAttributes,
    ) -> <C::Sequencer as Sequencer>::FullChar {
        unsafe { *(&(C::full::<P>(*fg, *bg, a), c) as *const _ as *const _) }
    }
    #[inline(always)]
    fn fgchar<P: AnsiPixel, C: Converter>(
        p: &P::Repr,
        c: Char,
        a: ColorAttributes,
    ) -> <C::Sequencer as Sequencer>::HalfChar {
        unsafe { *(&(C::fg::<P>(*p, a), c) as *const _ as *const _) }
    }
    #[inline(always)]
    fn bgchar<P: AnsiPixel, C: Converter>(
        p: &P::Repr,
        c: Char,
        a: ColorAttributes,
    ) -> <C::Sequencer as Sequencer>::HalfChar {
        unsafe { *(&(C::bg::<P>(*p, a), c) as *const _ as *const _) }
    }

    /// Implementation detail. whether fg or bg comes first in iteration
    pub trait Order<P: AnsiPixel, C: Converter>: Sized {
        fn full(p: (&P::Repr, &P::Repr), c: Char, a: ColorAttributes) -> <C::Sequencer as Sequencer>::FullChar;
        fn half(p: &P::Repr, c: Char, a: ColorAttributes) -> <C::Sequencer as Sequencer>::HalfChar;
    }
    /// Char is upper half of cell
    pub struct Upper;
    /// Char is lower half of cell
    pub struct Lower;
    /// Iterator over a full character row (2 pixels per character cell)
    pub struct FullAnsiRow<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> {
        pub(crate) iter: Zip<Iter<'a, P::Repr>, Iter<'a, P::Repr>>,
        pub(crate) char: Char,
        pub(crate) attrs: ColorAttributes,
        pub(crate) phantom: PhantomData<(C, O)>,
    }
    /// Iterator over a half character row (1 pixel per character cell)
    pub struct HalfAnsiRow<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> {
        pub(crate) iter: Iter<'a, P::Repr>,
        pub(crate) char: Char,
        pub(crate) attrs: ColorAttributes,
        pub(crate) phantom: PhantomData<(C, O)>,
    }
    /// Iterator over a row of characters
    pub enum AnsiRow<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> {
        Full(FullAnsiRow<'a, P, C, O>),
        Half(HalfAnsiRow<'a, P, C, O>),
    }
    /// Iterator over rows of characters in an image
    pub struct AnsiRows<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> {
        pub(crate) iter: ChunksExact<'a, P::Repr>,
        pub(crate) char: Char,
        pub(crate) attrs: ColorAttributes,
        pub(crate) phantom: PhantomData<(C, O)>,
    }

    impl<P: AnsiPixel, C: Converter> Order<P, C> for Upper {
        #[inline(always)]
        fn full(
            (fg, bg): (&<P>::Repr, &<P>::Repr),
            c: Char,
            a: ColorAttributes,
        ) -> <<C as Converter>::Sequencer as Sequencer>::FullChar {
            fullchar::<P, C>(fg, bg, c, a)
        }
        #[inline(always)]
        fn half(p: &<P>::Repr, c: Char, a: ColorAttributes) -> <<C as Converter>::Sequencer as Sequencer>::HalfChar {
            fgchar::<P, C>(p, c, a)
        }
    }

    impl<P: AnsiPixel, C: Converter> Order<P, C> for Lower {
        #[inline(always)]
        fn full(
            (bg, fg): (&<P>::Repr, &<P>::Repr),
            c: Char,
            a: ColorAttributes,
        ) -> <<C as Converter>::Sequencer as Sequencer>::FullChar {
            fullchar::<P, C>(fg, bg, c, a)
        }
        #[inline(always)]
        fn half(p: &<P>::Repr, c: Char, a: ColorAttributes) -> <<C as Converter>::Sequencer as Sequencer>::HalfChar {
            bgchar::<P, C>(p, c, a)
        }
    }

    impl<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> Iterator for FullAnsiRow<'a, P, C, O> {
        type Item = <<C as Converter>::Sequencer as Sequencer>::FullChar;
        #[inline(always)]
        fn next(&mut self) -> Option<Self::Item> { self.iter.next().map(|p| O::full(p, self.char, self.attrs)) }
        #[inline(always)]
        fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }
    }
    impl<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> DoubleEndedIterator for FullAnsiRow<'a, P, C, O> {
        #[inline(always)]
        fn next_back(&mut self) -> Option<Self::Item> { self.iter.next_back().map(|p| O::full(p, self.char, self.attrs)) }
    }
    impl<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> ExactSizeIterator for FullAnsiRow<'a, P, C, O> {
        #[inline(always)]
        fn len(&self) -> usize { self.iter.len() }
    }
    impl<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> FusedIterator for FullAnsiRow<'a, P, C, O> {}

    impl<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> Iterator for HalfAnsiRow<'a, P, C, O> {
        type Item = <<C as Converter>::Sequencer as Sequencer>::HalfChar;
        #[inline(always)]
        fn next(&mut self) -> Option<Self::Item> { self.iter.next().map(|p| O::half(p, self.char, self.attrs)) }
        #[inline(always)]
        fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }
    }
    impl<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> DoubleEndedIterator for HalfAnsiRow<'a, P, C, O> {
        #[inline(always)]
        fn next_back(&mut self) -> Option<Self::Item> { self.iter.next_back().map(|p| O::half(p, self.char, self.attrs)) }
    }
    impl<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> ExactSizeIterator for HalfAnsiRow<'a, P, C, O> {
        #[inline(always)]
        fn len(&self) -> usize { self.iter.len() }
    }
    impl<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> FusedIterator for HalfAnsiRow<'a, P, C, O> {}

    impl<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> Iterator for AnsiRows<'a, P, C, O> {
        type Item = AnsiRow<'a, P, C, O>;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            match (self.iter.next(), self.iter.next()) {
                (Some(a), Some(b)) => Some(AnsiRow::Full(FullAnsiRow {
                    iter: zip(a, b),
                    char: self.char,
                    attrs: self.attrs,
                    phantom: PhantomData,
                })),
                (Some(a), None) => Some(AnsiRow::Half(HalfAnsiRow {
                    iter: a.iter(),
                    char: self.char,
                    attrs: self.attrs,
                    phantom: PhantomData,
                })),
                _ => None,
            }
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let h = self.iter.size_hint();
            (div_ceil2(h.0), h.1.map(div_ceil2))
        }
    }
    impl<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> ExactSizeIterator for AnsiRows<'a, P, C, O> {
        #[inline(always)]
        fn len(&self) -> usize { div_ceil2(self.iter.len()) }
    }
    impl<'a, P: AnsiPixel, C: Converter, O: Order<P, C>> FusedIterator for AnsiRows<'a, P, C, O> {}
}
pub use iter::*;
