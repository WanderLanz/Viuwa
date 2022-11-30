//! Pixels to Ansi color sequences as Iterators, and color conversion functions
//!
//! Use `PixelConverter` to convert pixels to ansi color sequences

#[cfg(feature = "fir")]
use fast_image_resize as fir;
use image::{Luma, Rgb, Rgba};

#[cfg(feature = "fir")]
use super::*;
use crate::viuwa::ColorAttributes;

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

/// Base trait for converting a raw pixel value into an ansi color.
pub trait RawPixel: Sized {
    #[cfg(feature = "fir")]
    fn fir_dimensions<'a, P: Pixel>(
        img: &'a image::ImageBuffer<P, Vec<u8>>,
    ) -> Result<(core::num::NonZeroU32, core::num::NonZeroU32)> {
        match (core::num::NonZeroU32::new(img.width()), core::num::NonZeroU32::new(img.height())) {
            (Some(w), Some(h)) => Ok((w, h)),
            _ => Err(anyhow::anyhow!("Image dimensions are zero")),
        }
    }
    #[cfg(feature = "fir")]
    fn fir_view_image<'a, P: Pixel>(img: &'a image::ImageBuffer<P, Vec<u8>>) -> Result<fir::DynamicImageView<'a>>;
    #[cfg(feature = "fir")]
    fn fir_view_image_mut<'a, P: Pixel>(img: &'a mut image::ImageBuffer<P, Vec<u8>>)
        -> Result<fir::DynamicImageViewMut<'a>>;
    const CHANNELS: usize;
    const CHANNELS_F32: usize = Self::CHANNELS * core::mem::size_of::<f32>();
    type Repr: Clone + Copy + Send + Sync + Sized;
    fn to_24bit_color(p: Self::Repr, a: &ColorAttributes) -> [u8; 3];
    fn to_24bit_gray(p: Self::Repr, a: &ColorAttributes) -> [u8; 3];
    fn to_8bit_color(p: Self::Repr, a: &ColorAttributes) -> u8;
    fn to_8bit_gray(p: Self::Repr, a: &ColorAttributes) -> u8;
}

impl RawPixel for Rgba<u8> {
    #[cfg(feature = "fir")]
    fn fir_view_image<'a, P: Pixel>(img: &'a image::ImageBuffer<P, Vec<u8>>) -> Result<fir::DynamicImageView<'a>> {
        let (w, h) = Self::fir_dimensions(img)?;
        Ok(fir::DynamicImageView::U8x4(fir::ImageView::from_buffer(w, h, img)?))
    }
    #[cfg(feature = "fir")]
    fn fir_view_image_mut<'a, P: Pixel>(
        img: &'a mut image::ImageBuffer<P, Vec<u8>>,
    ) -> Result<fir::DynamicImageViewMut<'a>> {
        let (w, h) = Self::fir_dimensions(img)?;
        Ok(fir::DynamicImageViewMut::U8x4(fir::ImageViewMut::from_buffer(w, h, img)?))
    }
    const CHANNELS: usize = 4;
    type Repr = [u8; 4];
    #[inline(always)]
    fn to_24bit_color([r, g, b, _]: Self::Repr, _a: &ColorAttributes) -> [u8; 3] { [r, g, b] }
    #[inline(always)]
    fn to_24bit_gray([r, g, b, _]: Self::Repr, _a: &ColorAttributes) -> [u8; 3] {
        let v = luma([r, g, b]);
        [v, v, v]
    }
    #[inline(always)]
    fn to_8bit_color([r, g, b, _]: Self::Repr, a: &ColorAttributes) -> u8 { rgb_to_256([r, g, b], a) }
    #[inline(always)]
    fn to_8bit_gray([r, g, b, _]: Self::Repr, _a: &ColorAttributes) -> u8 { gray_to_256(luma([r, g, b])) }
}

impl RawPixel for Rgb<u8> {
    #[cfg(feature = "fir")]
    fn fir_view_image<'a, P: Pixel>(img: &'a image::ImageBuffer<P, Vec<u8>>) -> Result<fir::DynamicImageView<'a>> {
        let (w, h) = Self::fir_dimensions(img)?;
        Ok(fir::DynamicImageView::U8x3(fir::ImageView::from_buffer(w, h, img)?))
    }
    #[cfg(feature = "fir")]
    fn fir_view_image_mut<'a, P: Pixel>(
        img: &'a mut image::ImageBuffer<P, Vec<u8>>,
    ) -> Result<fir::DynamicImageViewMut<'a>> {
        let (w, h) = Self::fir_dimensions(img)?;
        Ok(fir::DynamicImageViewMut::U8x3(fir::ImageViewMut::from_buffer(w, h, img)?))
    }
    const CHANNELS: usize = 3;
    type Repr = [u8; 3];
    #[inline(always)]
    fn to_24bit_color(p: Self::Repr, _a: &ColorAttributes) -> [u8; 3] { p }
    #[inline(always)]
    fn to_24bit_gray(p: Self::Repr, _a: &ColorAttributes) -> [u8; 3] {
        let v = luma(p);
        [v, v, v]
    }
    #[inline(always)]
    fn to_8bit_color(p: Self::Repr, a: &ColorAttributes) -> u8 { rgb_to_256(p, a) }
    #[inline(always)]
    fn to_8bit_gray(p: Self::Repr, _a: &ColorAttributes) -> u8 { gray_to_256(luma(p)) }
}

impl RawPixel for Luma<u8> {
    #[cfg(feature = "fir")]
    fn fir_view_image<'a, P: Pixel>(img: &'a image::ImageBuffer<P, Vec<u8>>) -> Result<fir::DynamicImageView<'a>> {
        let (w, h) = Self::fir_dimensions(img)?;
        Ok(fir::DynamicImageView::U8(fir::ImageView::from_buffer(w, h, img)?))
    }
    #[cfg(feature = "fir")]
    fn fir_view_image_mut<'a, P: Pixel>(
        img: &'a mut image::ImageBuffer<P, Vec<u8>>,
    ) -> Result<fir::DynamicImageViewMut<'a>> {
        let (w, h) = Self::fir_dimensions(img)?;
        Ok(fir::DynamicImageViewMut::U8(fir::ImageViewMut::from_buffer(w, h, img)?))
    }
    const CHANNELS: usize = 1;
    type Repr = u8;
    #[inline(always)]
    fn to_24bit_color(p: Self::Repr, a: &ColorAttributes) -> [u8; 3] { Self::to_24bit_gray(p, a) }
    #[inline(always)]
    fn to_24bit_gray(p: Self::Repr, _: &ColorAttributes) -> [u8; 3] { [p, p, p] }
    #[inline(always)]
    fn to_8bit_color(p: Self::Repr, a: &ColorAttributes) -> u8 { Self::to_8bit_gray(p, a) }
    #[inline(always)]
    fn to_8bit_gray(p: Self::Repr, _: &ColorAttributes) -> u8 { gray_to_256(p) }
}

pub trait ColorWriter {
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

pub struct ColorWriter24bit;
impl ColorWriter for ColorWriter24bit {
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

pub struct ColorWriter8bit;
impl ColorWriter for ColorWriter8bit {
    const RESERVE_SIZE: usize = 23;
    type Repr = u8;
    #[inline(always)]
    fn fg(buf: &mut Vec<u8>, val: Self::Repr) { seq8(buf, val, b"\x1b[38;5;"); }
    #[inline(always)]
    fn bg(buf: &mut Vec<u8>, val: Self::Repr) { seq8(buf, val, b"\x1b[48;5;"); }
}

pub trait AnsiColor {
    type Writer: ColorWriter;
    fn to_repr<P: RawPixel>(p: P::Repr, a: &ColorAttributes) -> <Self::Writer as ColorWriter>::Repr;
    #[inline(always)]
    fn fg<'a, P: RawPixel>(buf: &mut Vec<u8>, p: P::Repr, a: &ColorAttributes) {
        Self::Writer::fg(buf, Self::to_repr::<P>(p, a))
    }
    #[inline(always)]
    fn bg<'a, P: RawPixel>(buf: &mut Vec<u8>, p: P::Repr, a: &ColorAttributes) {
        Self::Writer::bg(buf, Self::to_repr::<P>(p, a))
    }
}
pub struct Color24;
impl AnsiColor for Color24 {
    type Writer = ColorWriter24bit;
    #[inline(always)]
    fn to_repr<P: RawPixel>(p: P::Repr, a: &ColorAttributes) -> <Self::Writer as ColorWriter>::Repr {
        P::to_24bit_color(p, a)
    }
}
pub struct Color8;
impl AnsiColor for Color8 {
    type Writer = ColorWriter8bit;
    #[inline(always)]
    fn to_repr<P: RawPixel>(p: P::Repr, a: &ColorAttributes) -> <Self::Writer as ColorWriter>::Repr {
        P::to_8bit_color(p, a)
    }
}
pub struct Gray24;
impl AnsiColor for Gray24 {
    type Writer = ColorWriter24bit;
    #[inline(always)]
    fn to_repr<P: RawPixel>(p: P::Repr, a: &ColorAttributes) -> <Self::Writer as ColorWriter>::Repr {
        P::to_24bit_gray(p, a)
    }
}
pub struct Gray8;
impl AnsiColor for Gray8 {
    type Writer = ColorWriter8bit;
    #[inline(always)]
    fn to_repr<P: RawPixel>(p: P::Repr, a: &ColorAttributes) -> <Self::Writer as ColorWriter>::Repr { P::to_8bit_gray(p, a) }
}

pub struct PixelWriter;
impl PixelWriter {
    #[inline]
    pub fn fg<'a, P: RawPixel, C: AnsiColor>(buf: &mut Vec<u8>, p: P::Repr, a: &ColorAttributes) { C::fg::<P>(buf, p, a) }
    #[inline]
    pub fn bg<'a, P: RawPixel, C: AnsiColor>(buf: &mut Vec<u8>, p: P::Repr, a: &ColorAttributes) { C::bg::<P>(buf, p, a) }
}
