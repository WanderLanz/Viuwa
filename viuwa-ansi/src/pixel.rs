//! Core module for any image or pixel related functionality.
//!
//! For users of this crate, you only have to implementing the [`AnsiPixel`] trait to use [`AnsiImage`](super::image::AnsiImage) or [`AnsiImageIter`](super::image::AnsiImageIter).
//!
//! Any type implementing [`AnsiPixel`] can be converted into an ANSI sequence representing a [`ColorType`] as
//! a foreground or background color using a [`Converter`].
//!
//! A [`Converter`] is a trait that converts any [`AnsiPixel`] into ANSI sequences using a [`Sequencer`],
//! and each [`Converter`] converts to a singular [`ColorType`]. (e.g. [`ColorConverter`] converts an [`AnsiPixel`] to ANSI representing [`ColorType::Color`])
//!
//! A [`Sequencer`] is a trait that converts raw representable foreground and/or background color channels into ANSI sequences.
//! A [`RgbSequencer`] is a [`Sequencer`] that converts 24-bit (RGB) colors into ANSI sequences.
//! An [`AnsiSequencer`] is a [`Sequencer`] that converts 8-bit (ANSI 256) colors into ANSI sequences.
//!
use ::core::mem::transmute;
use viuwa_image::*;

use super::*;
use crate::color::*;

/// Converts raw color values
/// into foreground/background ansi sequences that can written to the terminal and displayed.
/// ```
/// use viuwa_ansi::RgbSequencer;
/// assert_eq!(RgbSequencer::fg([255, 128, 0]).as_slice(), "\x1b[38;2;255;128;000m".as_bytes());
/// assert_eq!(RgbSequencer::bg([255, 128, 0]).as_slice(), "\x1b[48;2;255;128;000m".as_bytes());
/// assert_eq!(RgbSequencer::full([255, 128, 0], [0, 128, 255]).as_slice(), "\x1b[38;2;255;128;000;48;2;000;128;255m".as_bytes());
/// ```
/// ### NOTES
/// every image pixel is written to the terminal as 1/2 of a terminal row by using
/// either the foreground or background color of a character, so we can save space by including both in the same sequence.
pub trait Sequencer: Sealed {
    /// The raw color channels that this sequencer can recognize as one defined color.
    type Raw: Bytes;
    /// A singular standalone foreground or background color sequence.
    type Half: Bytes;
    /// A full foreground and background color sequence.
    type Full: Bytes;
    /// The [`Half`](Sequencer::Half) sequence and a [`Char`] in bytes. (sequence + 4 u8's)
    type HalfChar: Bytes;
    /// The [`Full`](Sequencer::Full) sequence and a [`Char`] in bytes. (sequence + 4 u8's)
    type FullChar: Bytes;
    /// Convert a raw foreground color into an ANSI sequence.
    fn fg(raw: Self::Raw) -> Self::Half;
    /// Convert a raw background color into an ANSI sequence.
    fn bg(raw: Self::Raw) -> Self::Half;
    /// Convert a foreground and background `Single` into an ANSI sequence.
    fn full(fg: Self::Raw, bg: Self::Raw) -> Self::Full;
}

/// Use the static FMT_U8 array to convert a u8 into a 3 byte array of ascii base 10 digits.
/// ```
/// assert_eq!(fmt_u8(0), [b'0', b'0', b'0']);
/// ```
#[inline(always)]
fn fmt_u8(n: u8) -> [u8; 3] { FMT_U8[n as usize] }
const CSI: [u8; 2] = [b'\x1b', b'['];
const FG24: [u8; 5] = [b'3', b'8', b';', b'2', b';'];
const BG24: [u8; 5] = [b'4', b'8', b';', b'2', b';'];
const FG8: [u8; 5] = [b'3', b'8', b';', b'5', b';'];
const BG8: [u8; 5] = [b'4', b'8', b';', b'5', b';'];

/// The Sequencer that recognizes 24-bit (RGB) colors.
pub struct RgbSequencer;
impl Sealed for RgbSequencer {}
impl Sequencer for RgbSequencer {
    type Raw = [u8; 3];
    type Half = [u8; 19];
    type Full = [u8; 36];
    type HalfChar = [u8; 23];
    type FullChar = [u8; 40];
    #[inline]
    fn fg(raw: Self::Raw) -> Self::Half {
        let [r, g, b] = raw.map(fmt_u8);
        unsafe { transmute((CSI, FG24, r, b';', g, b';', b, b'm')) }
    }
    #[inline]
    fn bg(raw: Self::Raw) -> Self::Half {
        let [r, g, b] = raw.map(fmt_u8);
        unsafe { transmute((CSI, BG24, r, b';', g, b';', b, b'm')) }
    }
    #[inline]
    fn full(fg: Self::Raw, bg: Self::Raw) -> Self::Full {
        let [a, b, c] = fg.map(fmt_u8);
        let [d, e, f] = bg.map(fmt_u8);
        unsafe { transmute((CSI, FG24, a, b';', b, b';', c, b';', BG24, d, b';', e, b';', f, b'm')) }
    }
}
/// The Sequencer that recognizes 8-bit (ANSI 256) colors.
pub struct AnsiSequencer;
impl Sealed for AnsiSequencer {}
impl Sequencer for AnsiSequencer {
    type Raw = u8;
    type Half = [u8; 11];
    type Full = [u8; 20];
    type HalfChar = [u8; 15];
    type FullChar = [u8; 24];
    #[inline]
    fn fg(raw: Self::Raw) -> Self::Half { unsafe { transmute((CSI, FG8, fmt_u8(raw), b'm')) } }
    #[inline]
    fn bg(raw: Self::Raw) -> Self::Half { unsafe { transmute((CSI, BG8, fmt_u8(raw), b'm')) } }
    #[inline]
    fn full(fg: Self::Raw, bg: Self::Raw) -> Self::Full {
        unsafe { transmute((CSI, FG8, fmt_u8(fg), b';', BG8, fmt_u8(bg), b'm')) }
    }
}

/// Converts any pixel implementing [`AnsiPixel`] into an ANSI foreground and/or background sequence representing its corresponding [`ColorType`]
pub trait Converter: Sealed {
    /// The [`Sequencer`] that this converter uses to convert pixels.
    type Sequencer: Sequencer;
    /// Convert a pixel into raw color channels that can be used by the [`Sequencer`].
    fn convert<P: AnsiPixel>(pixel: P::Repr, attributes: ColorAttributes) -> <Self::Sequencer as Sequencer>::Raw;
    /// Convert a pixel into a foreground color sequence.
    #[inline(always)]
    fn fg<P: AnsiPixel>(pixel: P::Repr, attributes: ColorAttributes) -> <Self::Sequencer as Sequencer>::Half {
        Self::Sequencer::fg(Self::convert::<P>(pixel, attributes))
    }
    /// Convert a pixel into a background color sequence.
    #[inline(always)]
    fn bg<P: AnsiPixel>(pixel: P::Repr, attributes: ColorAttributes) -> <Self::Sequencer as Sequencer>::Half {
        Self::Sequencer::bg(Self::convert::<P>(pixel, attributes))
    }
    /// Convert pixels into a foreground and background color sequence.
    #[inline(always)]
    fn full<P: AnsiPixel>(fg: P::Repr, bg: P::Repr, attributes: ColorAttributes) -> <Self::Sequencer as Sequencer>::Full {
        Self::Sequencer::full(Self::convert::<P>(fg, attributes), Self::convert::<P>(bg, attributes))
    }
}
/// Converter to 24-bit (RGB) color.
pub struct ColorConverter;
impl Sealed for ColorConverter {}
impl Converter for ColorConverter {
    type Sequencer = RgbSequencer;
    #[inline(always)]
    fn convert<P: AnsiPixel>(p: P::Repr, a: ColorAttributes) -> <Self::Sequencer as Sequencer>::Raw { P::to_rgb(p, a) }
}
/// Converter to 8-bit (ANSI 256) color.
pub struct AnsiColorConverter;
impl Sealed for AnsiColorConverter {}
impl Converter for AnsiColorConverter {
    type Sequencer = AnsiSequencer;
    #[inline(always)]
    fn convert<P: AnsiPixel>(p: P::Repr, a: ColorAttributes) -> <Self::Sequencer as Sequencer>::Raw { P::to_256(p, a) }
}
/// Converter to 24-bit (RGB) grayscale colors.
pub struct GrayConverter;
impl Sealed for GrayConverter {}
impl Converter for GrayConverter {
    type Sequencer = RgbSequencer;
    #[inline(always)]
    fn convert<P: AnsiPixel>(p: P::Repr, a: ColorAttributes) -> <Self::Sequencer as Sequencer>::Raw { [P::to_luma(p, a); 3] }
}
/// Converter to 8-bit (ANSI 256) grayscale colors.
pub struct AnsiGrayConverter;
impl Sealed for AnsiGrayConverter {}
impl Converter for AnsiGrayConverter {
    type Sequencer = AnsiSequencer;
    #[inline(always)]
    fn convert<P: AnsiPixel>(p: P::Repr, a: ColorAttributes) -> <Self::Sequencer as Sequencer>::Raw {
        gray_to_ansi(P::to_luma(p, a))
    }
}

/// Pixel types usable with a [`Converter`]
pub trait AnsiPixel: Pixel {
    /// Convert Repr to 24-bit RGB value.
    fn to_rgb(p: Self::Repr, a: ColorAttributes) -> [u8; 3];
    /// Convert Repr to 8-bit grayscale value.
    fn to_luma(p: Self::Repr, a: ColorAttributes) -> u8;
    /// Convert Repr to 8-bit ANSI 256 color value.
    fn to_256(p: Self::Repr, a: ColorAttributes) -> u8;
}

/// Predefined 24-bit RGB pixel usable with a [`Converter`]
pub struct ColorPixel;
impl Pixel for ColorPixel {
    type Scalar = u8;
    type Repr = [u8; 3];
}
impl AnsiPixel for ColorPixel {
    #[inline(always)]
    fn to_rgb(p: Self::Repr, _: ColorAttributes) -> [u8; 3] { p }
    #[inline(always)]
    fn to_luma(p: Self::Repr, _: ColorAttributes) -> u8 { luma(p) }
    #[inline(always)]
    fn to_256(p: Self::Repr, a: ColorAttributes) -> u8 { rgb_to_ansi(p, a) }
}
/// Predefined 8-bit (ANSI 256) color pixel usable with a [`Converter`]
pub struct AnsiColorPixel;
impl Pixel for AnsiColorPixel {
    type Scalar = u8;
    type Repr = u8;
}
impl AnsiPixel for AnsiColorPixel {
    #[inline(always)]
    fn to_rgb(p: Self::Repr, _: ColorAttributes) -> [u8; 3] { ansi_to_rgb(p) }
    #[inline(always)]
    fn to_luma(p: Self::Repr, _: ColorAttributes) -> u8 { luma(ansi_to_rgb(p)) }
    #[inline(always)]
    fn to_256(p: Self::Repr, _: ColorAttributes) -> u8 { p }
}
/// Predefined 24-bit grayscale pixel usable with a [`Converter`]
pub struct GrayPixel;
impl Pixel for GrayPixel {
    type Scalar = u8;
    type Repr = u8;
}
impl AnsiPixel for GrayPixel {
    #[inline(always)]
    fn to_rgb(p: Self::Repr, _: ColorAttributes) -> [u8; 3] { [p; 3] }
    #[inline(always)]
    fn to_luma(p: Self::Repr, _: ColorAttributes) -> u8 { p }
    #[inline(always)]
    fn to_256(p: Self::Repr, _: ColorAttributes) -> u8 { gray_to_ansi(p) }
}
/// Predefined 8-bit (ANSI 256) grayscale pixel usable with a [`Converter`]
pub struct AnsiGrayPixel;
impl Pixel for AnsiGrayPixel {
    type Scalar = u8;
    type Repr = u8;
}
impl AnsiPixel for AnsiGrayPixel {
    #[inline(always)]
    fn to_rgb(p: Self::Repr, _: ColorAttributes) -> [u8; 3] { ansi_to_rgb(p) }
    #[inline(always)]
    fn to_luma(p: Self::Repr, _: ColorAttributes) -> u8 { p }
    #[inline(always)]
    fn to_256(p: Self::Repr, _: ColorAttributes) -> u8 { p }
}

#[cfg(feature = "image")]
mod compat_image {
    use ::image::{Luma, Rgb};

    use super::*;

    // We can only guarantee the expected behavior of Pixel<u8>
    impl AnsiPixel for Rgb<u8> {
        #[inline(always)]
        fn to_rgb(p: Self::Repr, _: ColorAttributes) -> [u8; 3] { p }
        #[inline(always)]
        fn to_luma(p: Self::Repr, _: ColorAttributes) -> u8 { luma(p) }
        #[inline(always)]
        fn to_256(p: Self::Repr, a: ColorAttributes) -> u8 { rgb_to_ansi(p, a) }
    }
    impl AnsiPixel for Luma<u8> {
        #[inline(always)]
        fn to_rgb(p: Self::Repr, _: ColorAttributes) -> [u8; 3] { [p; 3] }
        #[inline(always)]
        fn to_luma(p: Self::Repr, _: ColorAttributes) -> u8 { p }
        #[inline(always)]
        fn to_256(p: Self::Repr, _: ColorAttributes) -> u8 { gray_to_ansi(p) }
    }
}
