//! Pixels to Ansi color sequences as Iterators, and color conversion functions
//!
//! Use `PixelConverter` to convert pixels to ansi color sequences

use super::*;

macro_rules! uninit {
    () => {
        #[allow(invalid_value)]
        unsafe {
            ::core::mem::MaybeUninit::uninit().assume_init()
        }
    };
    ($t:ty) => {
        #[allow(invalid_value)]
        unsafe {
            ::core::mem::MaybeUninit::<$t>::uninit().assume_init()
        }
    };
}

#[cfg_attr(feature = "clap", derive(::clap::ValueEnum))]
#[derive(Debug, Clone, Copy, Default)]
pub enum ColorType {
    #[default]
    #[cfg_attr(feature = "clap", value(name = "color", alias = "truecolor"))]
    Color,
    #[cfg_attr(feature = "clap", value(name = "ansi_color", alias = "ansi"))]
    AnsiColor,
    #[cfg_attr(feature = "clap", value(name = "gray", alias = "grayscale"))]
    Gray,
    #[cfg_attr(feature = "clap", value(name = "ansi_gray", alias = "agray"))]
    AnsiGray,
}

impl ColorType {
    /// Cycle through the color types
    #[inline]
    pub fn cycle(&self) -> ColorType {
        match self {
            ColorType::Color => ColorType::AnsiColor,
            ColorType::AnsiColor => ColorType::Gray,
            ColorType::Gray => ColorType::AnsiGray,
            ColorType::AnsiGray => ColorType::Color,
        }
    }
    /// Can the colortype represent different hues?
    #[inline]
    pub fn is_color(&self) -> bool {
        match self {
            ColorType::Color | ColorType::AnsiColor => true,
            ColorType::Gray | ColorType::AnsiGray => false,
        }
    }
    /// Can the colortype only represent grayscale?
    #[inline]
    pub fn is_gray(&self) -> bool { !self.is_color() }
    /// Does the colortype use 24 bits? (e.g. rgb, gray rgb)
    #[inline]
    pub fn is_24bit(&self) -> bool {
        match self {
            ColorType::Color | ColorType::Gray => true,
            ColorType::AnsiColor | ColorType::AnsiGray => false,
        }
    }
    /// Does the colortype use 8 bits? (e.g. rgb, gray rgb)
    #[inline]
    pub fn is_8bit(&self) -> bool { !self.is_24bit() }
}

/// Wrapper around possibly user-controlled color attributes
#[derive(Debug, Clone, Copy)]
pub struct ColorAttributes {
    /// luma correct as a color distance threshold
    pub luma_correct: u32,
}

impl ColorAttributes {
    /// luma correct is 0..=100, 100 is the highest luma correct
    // for n and f(luma_correct) = ((100 - luma_correct)^n / 100^(n-1)), as n increases, the luma correct becomes less aggressive
    // distance threshold = (MAX_COLOR_DISTANCE / 100) * ((100 - luma_correct)^3 / 100^2)
    pub fn new(luma_correct: u32) -> Self {
        Self { luma_correct: (((100 - luma_correct).pow(3) / 10000) as f32 * color::MAP_DIST_100) as u32 }
    }
}

macro_rules! color_enum {
    {
        $(#[$meta:meta])*
        $vis:vis enum $ident:ident {
            $($variant:ident = ($fg:literal, $bg:literal)),* $(,)?
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
                    $($variant => $crate::sgr!($fg)),*
                }
            }
            pub const fn bg(self) -> &'static str {
                use $ident::*;
                match self {
                    $($variant => $crate::sgr!($bg)),*
                }
            }
        }
    };
}
color_enum! {
/// ANSI 3-bit color presets for foreground and background.
/// See the [wikipedia](https://en.wikipedia.org/wiki/ANSI_escape_code#Colors) for more info.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum ColorPresets {
    black = ("30", "40"),
    red = ("31", "41"),
    green = ("32", "42"),
    yellow = ("33", "43"),
    blue = ("34", "44"),
    magenta = ("35", "45"),
    cyan = ("36", "46"),
    white = ("37", "47"),
    Black = ("90", "100"),
    Red = ("91", "101"),
    Green = ("92", "102"),
    Yellow = ("93", "103"),
    Blue = ("94", "104"),
    Magenta = ("95", "105"),
    Cyan = ("96", "106"),
    White = ("97", "107"),
}}

/// The maximum distance two RGB colors can have from one another
pub const MAX_COLOR_DISTANCE: u32 = 584_970_u32;
/// Coefficient used to transform a value within `0..=100` to color distance
pub const MAP_DIST_100: f32 = MAX_COLOR_DISTANCE as f32 / 100.;

/// Get the closest ANSI 256 (8-bit) color to the given 24-bit sRGB color.
#[inline]
pub fn rgb_to_ansi(c: [u8; 3], a: ColorAttributes) -> u8 {
    let xyz = rgb_to_ansi_direct(c);
    let gray = gray_to_ansi(luma(c));
    if dist(c, ansi_to_rgb(gray)) + a.luma_correct < dist(c, ansi_to_rgb(xyz)) {
        gray
    } else {
        xyz
    }
}

/// Grayscale u8 to ANSI 256 (8-bit) color.
#[inline(always)]
pub fn gray_to_ansi(c: u8) -> u8 { ANSI_GRAY[c as usize] }

/// ANSI 256 (8-bit) color to 24-bit RGB color.
#[inline(always)]
pub fn ansi_to_rgb(c: u8) -> [u8; 3] { ANSI_PALETTE[c as usize] }

/// Compute the luma of the given 24-bit sRGB color (sRGB -> Luma).
#[inline]
pub fn luma([r, g, b]: [u8; 3]) -> u8 { ((r as u32 * 2126 + g as u32 * 7152 + b as u32 * 722) / 10000) as u8 }

/// Get the distance between two 24-bit rgb colors.
/// 0..=584_970
#[inline]
pub const fn dist([r1, g1, b1]: [u8; 3], [r2, g2, b2]: [u8; 3]) -> u32 {
    let rmean = (r1 as u32 + r2 as u32) / 2;
    let r = (r1 as u32).abs_diff(r2 as u32);
    let g = (g1 as u32).abs_diff(g2 as u32);
    let b = (b1 as u32).abs_diff(b2 as u32);
    (((512 + rmean) * r * r) >> 8) + 4 * g * g + (((767 - rmean) * b * b) >> 8)
}

/// Get the closest 8-bit color in the 6x6x6 cube to the given 24-bit rgb color.
#[inline]
pub fn rgb_to_ansi_direct([r, g, b]: [u8; 3]) -> u8 {
    const MAP_0_255_0_5: f32 = 5.0 / 255.0;
    let r = (r as f32 * MAP_0_255_0_5).round() as u8;
    let g = (g as f32 * MAP_0_255_0_5).round() as u8;
    let b = (b as f32 * MAP_0_255_0_5).round() as u8;
    (36 * r + 6 * g + b) as u8 + 16
}
