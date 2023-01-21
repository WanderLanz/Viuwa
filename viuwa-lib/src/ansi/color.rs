//! Pixels to Ansi color sequences as Iterators, and color conversion functions
//!
//! Use `PixelConverter` to convert pixels to ansi color sequences

use super::*;

macro_rules! color_preset_enum {
    {
        $(#[$meta:meta])*
        $vis:vis enum $ident:ident {
            $($variant:ident = $value:literal),* $(,)?
        }
    } => {
        $(#[$meta])*
        $vis enum $ident {
            $($variant),*
        }
        impl $ident {
            pub const fn fg(self) -> &'static str {
                use $ident::*;
                match self {
                    $($variant => csi!($value, "m")),*
                }
            }
            pub const fn bg(self) -> &'static str {
                use $ident::*;
                match self {
                    $($variant => csi!($value, "m")),*
                }
            }
        }
    };
}
color_preset_enum! {
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum AnsiColorPresets {
    black = "30",
    red = "31",
    green = "32",
    yellow = "33",
    blue = "34",
    magenta = "35",
    cyan = "36",
    white = "37",
    Black = "90",
    Red = "91",
    Green = "92",
    Yellow = "93",
    Blue = "94",
    Magenta = "95",
    Cyan = "96",
    White = "97",
}}

pub const MAX_COLOR_DISTANCE: u32 = 584_970_u32;
pub const MAP_0_100_DIST: f32 = MAX_COLOR_DISTANCE as f32 / 100.;

/// 256-color palette as 24-bit RGB values. %18.75 of 4KB.
pub static EIGHT_BIT_PALETTE: [[u8; 3]; 256] = include!("256rgb.ron");

/// Closest 256 color to a given grayscale value. %6.25 of 4KB.
// thanks to [ansi_colours](https://crates.io/crates/ansi_colours)
#[rustfmt::skip]
pub static GRAY_TO_256: [u8; 256] = include!("gray256.ron");

/// Static u8 format lookup table. approx 4KB on 64-bit arch.
#[rustfmt::skip]
pub static FMT_U8: [&'static [u8]; 256] = include!("fmt_u8.ron");

/// Get the closest 8-bit color to the given 24-bit color.
#[inline]
pub fn rgb_to_256(c: [u8; 3], ca: &ColorAttributes) -> u8 {
    let xyz = rgb_in_256(c);
    let luma = gray_to_256(luma(c));
    if dist(c, rgb_from_256(luma)) + ca.luma_correct < dist(c, rgb_from_256(xyz)) {
        luma
    } else {
        xyz
    }
}

#[inline(always)]
pub fn gray_to_256(c: u8) -> u8 { GRAY_TO_256[c as usize] }

#[inline(always)]
pub fn rgb_from_256(c: u8) -> [u8; 3] { EIGHT_BIT_PALETTE[c as usize] }

/// Get the luma of the given 24-bit color (sRGB -> Luma).
#[inline]
pub fn luma([r, g, b]: [u8; 3]) -> u8 { ((r as u32 * 2126 + g as u32 * 7152 + b as u32 * 722) / 10000) as u8 }

/// Get the distance between two 24-bit colors.
/// 0..=584970
#[inline]
pub const fn dist([r1, g1, b1]: [u8; 3], [r2, g2, b2]: [u8; 3]) -> u32 {
    let rmean = (r1 as u32 + r2 as u32) / 2;
    let r = (r1 as u32).abs_diff(r2 as u32);
    let g = (g1 as u32).abs_diff(g2 as u32);
    let b = (b1 as u32).abs_diff(b2 as u32);
    (((512 + rmean) * r * r) >> 8) + 4 * g * g + (((767 - rmean) * b * b) >> 8)
}

/// Get the closest 8-bit color in the 6x6x6 cube to the given 24-bit color.
#[inline]
pub fn rgb_in_256([r, g, b]: [u8; 3]) -> u8 {
    const MAP_0_255_0_5: f32 = 5.0 / 255.0;
    let r = (r as f32 * MAP_0_255_0_5).round() as u8;
    let g = (g as f32 * MAP_0_255_0_5).round() as u8;
    let b = (b as f32 * MAP_0_255_0_5).round() as u8;
    (36 * r + 6 * g + b) as u8 + 16
}

#[cfg(feature = "fir")]
fn fir_dimensions<P: RawPixel>(img: ImageView<P>) -> Result<(core::num::NonZeroU32, core::num::NonZeroU32)> {
    match (core::num::NonZeroU32::new(img.width() as u32), core::num::NonZeroU32::new(img.height() as u32)) {
        (Some(w), Some(h)) => Ok((w, h)),
        _ => Err(anyhow::anyhow!("Image dimensions are zero")),
    }
}

/// Pixel types that can be used with this crate and `viuwa-ansi`
pub trait RawPixel: Sized {
    /// The representation of a pixel as a flat array of scalars with length 1 to 5 (e.g. `[u8; 3]`, `[u16; 4]`, etc.)
    type Repr: PixelRepr;
    /// Convert a pixel to a standard 24-bit sRGB value.
    fn to_rgb(p: Self::Repr, a: &ColorAttributes) -> [u8; 3];
    /// Convert a pixel to a grayscale value.
    fn to_luma(p: Self::Repr, a: &ColorAttributes) -> u8;
    /// Convert a pixel to an ANSI 256-color value.
    fn to_256(p: Self::Repr, a: &ColorAttributes) -> u8;
}

/// Pixel types that can be used with this crate and `viuwa-ansi`, compatible with all features.
pub trait CompatiblePixel: RawPixel
where
    <Self::Repr as PixelRepr>::Scalar: CompatibleScalar,
    Self::Repr: CompatiblePixelRepr,
{
}
impl<P: RawPixel> CompatiblePixel for P
where
    <P::Repr as PixelRepr>::Scalar: CompatibleScalar,
    P::Repr: CompatiblePixelRepr,
{
}

/// Check if the pixel Repr's channel count is compatible with the pixel type in the image crate.
#[cfg(feature = "image")]
#[doc(hidden)]
mod check_image_compat {
    trait ChannelCheck: RawPixel + ::image::Pixel {
        const CHECK: ();
    }
    impl<P: RawPixel + ::image::Pixel> ChannelCheck for P {
        const CHECK: () = {
            if <P as ::image::Pixel>::CHANNEL_COUNT as usize != <P as RawPixel>::CHANNELS {
                panic!("image::Pixel and RawPixel::Repr have different channel counts, perhaps you need to implement RawPixel for your own pixel type.");
            }
        };
    }
}

impl<T: Scalar> RawPixel for Rgb<T> {
    type Repr = [T; 3];
    #[inline(always)]
    fn to_rgb(p: Self::Repr, _a: &ColorAttributes) -> [u8; 3] { p }
    #[inline(always)]
    fn to_luma(p: Self::Repr, _a: &ColorAttributes) -> u8 { luma(p) }
    #[inline(always)]
    fn to_256(p: Self::Repr, a: &ColorAttributes) -> u8 { rgb_to_256(p, a) }
}

impl<T: Scalar> RawPixel for Luma<T> {
    type Repr = [T; 1];
    #[inline(always)]
    fn to_rgb([p]: Self::Repr, _a: &ColorAttributes) -> [u8; 3] { [p; 3] }
    #[inline(always)]
    fn to_luma([p]: Self::Repr, _a: &ColorAttributes) -> u8 { p }
    #[inline(always)]
    fn to_256([p]: Self::Repr, _a: &ColorAttributes) -> u8 { gray_to_256(p) }
}

pub trait AnsiColorWriter {
    /// Reserve space for 2 color sequences (fg+bg) and 1 display character.
    const RESERVE_SIZE: usize;
    /// The representation of pixel data for this writer.
    type Repr: Clone + Copy + Send + Sync + Sized;
    fn fg(buf: &mut Vec<u8>, val: Self::Repr);
    fn bg(buf: &mut Vec<u8>, val: Self::Repr);
}

// extend_from_slice([u8;1]) instead of push(u8) when you can is faster for some reason when I benchmarked it, some loop optimizations I guess
#[inline]
fn seq24(buf: &mut Vec<u8>, [r, g, b]: [u8; 3], inducer: &[u8]) {
    buf.extend_from_slice(inducer);
    buf.extend_from_slice(FMT_U8[r as usize]);
    buf.extend_from_slice(b";");
    buf.extend_from_slice(FMT_U8[g as usize]);
    buf.extend_from_slice(b";");
    buf.extend_from_slice(FMT_U8[b as usize]);
    buf.extend_from_slice(b"m");
}

pub struct AnsiRgbWriter;
impl AnsiColorWriter for AnsiRgbWriter {
    const RESERVE_SIZE: usize = 39;
    type Repr = [u8; 3];
    #[inline(always)]
    fn fg(buf: &mut Vec<u8>, val: Self::Repr) { seq24(buf, val, b"\x1b[38;2;"); }
    #[inline(always)]
    fn bg(buf: &mut Vec<u8>, val: Self::Repr) { seq24(buf, val, b"\x1b[48;2;"); }
}

#[inline]
fn seq8(buf: &mut Vec<u8>, c: u8, inducer: &[u8]) {
    buf.extend_from_slice(inducer);
    buf.extend_from_slice(FMT_U8[c as usize]);
    buf.extend_from_slice(b"m");
}

pub struct Ansi256Writer;
impl AnsiColorWriter for Ansi256Writer {
    const RESERVE_SIZE: usize = 23;
    type Repr = u8;
    #[inline(always)]
    fn fg(buf: &mut Vec<u8>, val: Self::Repr) { seq8(buf, val, b"\x1b[38;5;"); }
    #[inline(always)]
    fn bg(buf: &mut Vec<u8>, val: Self::Repr) { seq8(buf, val, b"\x1b[48;5;"); }
}

pub trait AnsiColor {
    type Writer: AnsiColorWriter;
    /// Convert the raw pixel repr to the repr for the writer.
    fn repr_pixel<P: RawPixel>(p: P::Repr, a: &ColorAttributes) -> <Self::Writer as AnsiColorWriter>::Repr;
    #[inline(always)]
    /// Write the foreground color sequence to the buffer.
    fn fg(buf: &mut Vec<u8>, val: <Self::Writer as AnsiColorWriter>::Repr) { Self::Writer::fg(buf, val) }
    #[inline(always)]
    /// Write the background color sequence to the buffer.
    fn bg(buf: &mut Vec<u8>, val: <Self::Writer as AnsiColorWriter>::Repr) { Self::Writer::bg(buf, val) }
}
pub struct ColorRgb;
impl AnsiColor for ColorRgb {
    type Writer = AnsiRgbWriter;
    #[inline(always)]
    fn repr_pixel<P: RawPixel>(p: P::Repr, a: &ColorAttributes) -> <Self::Writer as AnsiColorWriter>::Repr {
        P::to_rgb(p, a)
    }
}
pub struct Color256;
impl AnsiColor for Color256 {
    type Writer = Ansi256Writer;
    #[inline(always)]
    fn repr_pixel<P: RawPixel>(p: P::Repr, a: &ColorAttributes) -> <Self::Writer as AnsiColorWriter>::Repr {
        P::to_256(p, a)
    }
}
pub struct GrayRgb;
impl AnsiColor for GrayRgb {
    type Writer = AnsiRgbWriter;
    #[inline(always)]
    fn repr_pixel<P: RawPixel>(p: P::Repr, a: &ColorAttributes) -> <Self::Writer as AnsiColorWriter>::Repr {
        [P::to_luma(p, a); 3]
    }
}
pub struct Gray256;
impl AnsiColor for Gray256 {
    type Writer = Ansi256Writer;
    #[inline(always)]
    fn repr_pixel<P: RawPixel>(p: P::Repr, a: &ColorAttributes) -> <Self::Writer as AnsiColorWriter>::Repr {
        gray_to_256(P::to_luma(p, a))
    }
}

pub struct PixelWriter;
impl PixelWriter {
    #[inline]
    pub fn fg<'a, P: RawPixel, C: AnsiColor>(buf: &mut Vec<u8>, p: P::Repr, a: &ColorAttributes) {
        C::fg(buf, C::repr_pixel::<P>(p, a))
    }
    #[inline]
    pub fn bg<'a, P: RawPixel, C: AnsiColor>(buf: &mut Vec<u8>, p: P::Repr, a: &ColorAttributes) {
        C::bg(buf, C::repr_pixel::<P>(p, a))
    }
}
