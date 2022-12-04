//! Pixels to Ansi color sequences as Iterators, and color conversion functions
//!
//! Use `PixelConverter` to convert pixels to ansi color sequences

#[cfg(feature = "fir")]
use fast_image_resize as fir;
use image::{Luma, Rgb};

#[cfg(feature = "fir")]
use super::*;
use crate::viuwa::ColorAttributes;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum AnsiColorPresets {
    black,
    red,
    green,
    yellow,
    blue,
    magenta,
    cyan,
    white,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}
impl AnsiColorPresets {
    pub const fn fg(self) -> &'static str {
        use AnsiColorPresets::*;
        match self {
            black => csi!("30m"),
            red => csi!("31m"),
            green => csi!("32m"),
            yellow => csi!("33m"),
            blue => csi!("34m"),
            magenta => csi!("35m"),
            cyan => csi!("36m"),
            white => csi!("37m"),
            Black => csi!("90m"),
            Red => csi!("91m"),
            Green => csi!("92m"),
            Yellow => csi!("93m"),
            Blue => csi!("94m"),
            Magenta => csi!("95m"),
            Cyan => csi!("96m"),
            White => csi!("97m"),
        }
    }
    pub const fn bg(self) -> &'static str {
        use AnsiColorPresets::*;
        match self {
            black => csi!("40m"),
            red => csi!("41m"),
            green => csi!("42m"),
            yellow => csi!("43m"),
            blue => csi!("44m"),
            magenta => csi!("45m"),
            cyan => csi!("46m"),
            white => csi!("47m"),
            Black => csi!("100m"),
            Red => csi!("101m"),
            Green => csi!("102m"),
            Yellow => csi!("103m"),
            Blue => csi!("104m"),
            Magenta => csi!("105m"),
            Cyan => csi!("106m"),
            White => csi!("107m"),
        }
    }
}

pub const MAX_COLOR_DISTANCE: u32 = 584_970_u32;
pub const MAP_0_100_DIST: f32 = MAX_COLOR_DISTANCE as f32 / 100.;

/// 256-color palette as 24-bit RGB values. %18.75 of 4KB.
pub static EIGHT_BIT_PALETTE: [[u8; 3]; 256] = include!("256rgb.rs.txt");

/// Closest 256 color to a given grayscale value. %6.25 of 4KB.
// thanks to [ansi_colours](https://crates.io/crates/ansi_colours)
#[rustfmt::skip]
pub static GRAY_TO_256: [u8; 256] = include!("gray256.rs.txt");

/// Static u8 format lookup table. approx 4KB on 64-bit arch.
#[rustfmt::skip]
pub static FMT_U8: [&'static [u8]; 256] = include!("fmt_u8.rs.txt");

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
fn fir_dimensions<'a, P: Pixel>(
    img: &'a image::ImageBuffer<P, Vec<u8>>,
) -> Result<(core::num::NonZeroU32, core::num::NonZeroU32)> {
    match (core::num::NonZeroU32::new(img.width()), core::num::NonZeroU32::new(img.height())) {
        (Some(w), Some(h)) => Ok((w, h)),
        _ => Err(anyhow::anyhow!("Image dimensions are zero")),
    }
}

/// Base trait for converting a raw pixel value into an ansi color.
pub trait RawPixel: Sized {
    #[cfg(feature = "fir")]
    fn fir_view_image<'a, P: Pixel>(img: &'a image::ImageBuffer<P, Vec<u8>>) -> Result<fir::DynamicImageView<'a>>;
    #[cfg(feature = "fir")]
    fn fir_view_image_mut<'a, P: Pixel>(img: &'a mut image::ImageBuffer<P, Vec<u8>>)
        -> Result<fir::DynamicImageViewMut<'a>>;
    const CHANNELS: usize;
    type Repr: Clone + Copy + Send + Sync + Sized;
    fn to_rgb(p: Self::Repr, a: &ColorAttributes) -> [u8; 3];
    fn to_luma(p: Self::Repr, a: &ColorAttributes) -> u8;
    fn to_256(p: Self::Repr, a: &ColorAttributes) -> u8;
}

impl RawPixel for Rgb<u8> {
    #[cfg(feature = "fir")]
    fn fir_view_image<'a, P: Pixel>(img: &'a image::ImageBuffer<P, Vec<u8>>) -> Result<fir::DynamicImageView<'a>> {
        let (w, h) = fir_dimensions(img)?;
        Ok(fir::DynamicImageView::U8x3(fir::ImageView::from_buffer(w, h, img)?))
    }
    #[cfg(feature = "fir")]
    fn fir_view_image_mut<'a, P: Pixel>(
        img: &'a mut image::ImageBuffer<P, Vec<u8>>,
    ) -> Result<fir::DynamicImageViewMut<'a>> {
        let (w, h) = fir_dimensions(img)?;
        Ok(fir::DynamicImageViewMut::U8x3(fir::ImageViewMut::from_buffer(w, h, img)?))
    }
    const CHANNELS: usize = 3;
    type Repr = [u8; 3];
    #[inline(always)]
    fn to_rgb(p: Self::Repr, _a: &ColorAttributes) -> [u8; 3] { p }
    #[inline(always)]
    fn to_luma(p: Self::Repr, _a: &ColorAttributes) -> u8 { luma(p) }
    #[inline(always)]
    fn to_256(p: Self::Repr, a: &ColorAttributes) -> u8 { rgb_to_256(p, a) }
}

impl RawPixel for Luma<u8> {
    #[cfg(feature = "fir")]
    fn fir_view_image<'a, P: Pixel>(img: &'a image::ImageBuffer<P, Vec<u8>>) -> Result<fir::DynamicImageView<'a>> {
        let (w, h) = fir_dimensions(img)?;
        Ok(fir::DynamicImageView::U8(fir::ImageView::from_buffer(w, h, img)?))
    }
    #[cfg(feature = "fir")]
    fn fir_view_image_mut<'a, P: Pixel>(
        img: &'a mut image::ImageBuffer<P, Vec<u8>>,
    ) -> Result<fir::DynamicImageViewMut<'a>> {
        let (w, h) = fir_dimensions(img)?;
        Ok(fir::DynamicImageViewMut::U8(fir::ImageViewMut::from_buffer(w, h, img)?))
    }
    const CHANNELS: usize = 1;
    type Repr = u8;
    #[inline(always)]
    fn to_rgb(p: Self::Repr, _a: &ColorAttributes) -> [u8; 3] { [p; 3] }
    #[inline(always)]
    fn to_luma(p: Self::Repr, _a: &ColorAttributes) -> u8 { p }
    #[inline(always)]
    fn to_256(p: Self::Repr, _a: &ColorAttributes) -> u8 { gray_to_256(p) }
}

pub trait AnsiColorWriter {
    /// Reserve space for 2 color sequences + 1 character.
    const RESERVE_SIZE: usize;
    /// The representation of pixel data for this writer.
    type Repr: Clone + Copy + Send + Sync + Sized;
    fn fg(buf: &mut Vec<u8>, val: Self::Repr);
    fn bg(buf: &mut Vec<u8>, val: Self::Repr);
}

// extend_from_slice([u8;1]) instead of push(u8) when you can is faster for some reason when I benchmarked it, some optimizations I guess
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
