/// Pixels to Ansi color sequences as Iterators, and color conversion functions
///
/// Use `PixelConverter` to convert pixels to ansi color sequences
use image::{Luma, Rgb};

use crate::viuwa::ColorAttributes;

pub const MAX_COLOR_DISTANCE: u32 = 584_970_u32;
pub const MAP_0_100_DIST: f32 = MAX_COLOR_DISTANCE as f32 / 100.0;
/// Get the closest 8-bit color to the given 24-bit color.
pub fn rgb_to_256(c: [u8; 3], ca: &ColorAttributes) -> u8 {
    let xyz = rgb_in_256(c);
    let luma = GRAY_TO_256[luma(c) as usize];
    if dist(c, EIGHT_BIT_PALETTE[luma as usize]) + ca.luma_correct < dist(c, EIGHT_BIT_PALETTE[xyz as usize]) {
        luma
    } else {
        xyz
    }
}

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

const MAP_0_255_0_5: f32 = 5.0 / 255.0;
/// Get the closest 8-bit color in the 6x6x6 cube to the given 24-bit color.
#[inline]
pub fn rgb_in_256([r, g, b]: [u8; 3]) -> u8 {
    let r = (r as f32 * MAP_0_255_0_5).round() as u8;
    let g = (g as f32 * MAP_0_255_0_5).round() as u8;
    let b = (b as f32 * MAP_0_255_0_5).round() as u8;
    (36 * r + 6 * g + b) as u8 + 16
}

/// 256-color palette as 24-bit RGB values.
#[rustfmt::skip]
pub static EIGHT_BIT_PALETTE: [[u8;3]; 256] = [
    // unused, because they can be overriden
    [0x00, 0x00, 0x00],[0x00, 0x00, 0x00],[0x00, 0x00, 0x00],[0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00],[0x00, 0x00, 0x00],[0x00, 0x00, 0x00],[0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00],[0x00, 0x00, 0x00],[0x00, 0x00, 0x00],[0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00],[0x00, 0x00, 0x00],[0x00, 0x00, 0x00],[0x00, 0x00, 0x00],

    // 6×6×6 cube, RGB = XYZ
    [0x00,0x00,0x00], [0x00,0x00,0x5F], [0x00,0x00,0x87], [0x00,0x00,0xAF], [0x00,0x00,0xD7], [0x00,0x00,0xFF],
    [0x00,0x5F,0x00], [0x00,0x5F,0x5F], [0x00,0x5F,0x87], [0x00,0x5F,0xAF], [0x00,0x5F,0xD7], [0x00,0x5F,0xFF],
    [0x00,0x87,0x00], [0x00,0x87,0x5F], [0x00,0x87,0x87], [0x00,0x87,0xAF], [0x00,0x87,0xD7], [0x00,0x87,0xFF],
    [0x00,0xAF,0x00], [0x00,0xAF,0x5F], [0x00,0xAF,0x87], [0x00,0xAF,0xAF], [0x00,0xAF,0xD7], [0x00,0xAF,0xFF],
    [0x00,0xD7,0x00], [0x00,0xD7,0x5F], [0x00,0xD7,0x87], [0x00,0xD7,0xAF], [0x00,0xD7,0xD7], [0x00,0xD7,0xFF],
    [0x00,0xFF,0x00], [0x00,0xFF,0x5F], [0x00,0xFF,0x87], [0x00,0xFF,0xAF], [0x00,0xFF,0xD7], [0x00,0xFF,0xFF],

    [0x5F,0x00,0x00], [0x5F,0x00,0x5F], [0x5F,0x00,0x87], [0x5F,0x00,0xAF], [0x5F,0x00,0xD7], [0x5F,0x00,0xFF],
    [0x5F,0x5F,0x00], [0x5F,0x5F,0x5F], [0x5F,0x5F,0x87], [0x5F,0x5F,0xAF], [0x5F,0x5F,0xD7], [0x5F,0x5F,0xFF],
    [0x5F,0x87,0x00], [0x5F,0x87,0x5F], [0x5F,0x87,0x87], [0x5F,0x87,0xAF], [0x5F,0x87,0xD7], [0x5F,0x87,0xFF],
    [0x5F,0xAF,0x00], [0x5F,0xAF,0x5F], [0x5F,0xAF,0x87], [0x5F,0xAF,0xAF], [0x5F,0xAF,0xD7], [0x5F,0xAF,0xFF],
    [0x5F,0xD7,0x00], [0x5F,0xD7,0x5F], [0x5F,0xD7,0x87], [0x5F,0xD7,0xAF], [0x5F,0xD7,0xD7], [0x5F,0xD7,0xFF],
    [0x5F,0xFF,0x00], [0x5F,0xFF,0x5F], [0x5F,0xFF,0x87], [0x5F,0xFF,0xAF], [0x5F,0xFF,0xD7], [0x5F,0xFF,0xFF],

    [0x87,0x00,0x00], [0x87,0x00,0x5F], [0x87,0x00,0x87], [0x87,0x00,0xAF], [0x87,0x00,0xD7], [0x87,0x00,0xFF],
    [0x87,0x5F,0x00], [0x87,0x5F,0x5F], [0x87,0x5F,0x87], [0x87,0x5F,0xAF], [0x87,0x5F,0xD7], [0x87,0x5F,0xFF],
    [0x87,0x87,0x00], [0x87,0x87,0x5F], [0x87,0x87,0x87], [0x87,0x87,0xAF], [0x87,0x87,0xD7], [0x87,0x87,0xFF],
    [0x87,0xAF,0x00], [0x87,0xAF,0x5F], [0x87,0xAF,0x87], [0x87,0xAF,0xAF], [0x87,0xAF,0xD7], [0x87,0xAF,0xFF],
    [0x87,0xD7,0x00], [0x87,0xD7,0x5F], [0x87,0xD7,0x87], [0x87,0xD7,0xAF], [0x87,0xD7,0xD7], [0x87,0xD7,0xFF],
    [0x87,0xFF,0x00], [0x87,0xFF,0x5F], [0x87,0xFF,0x87], [0x87,0xFF,0xAF], [0x87,0xFF,0xD7], [0x87,0xFF,0xFF],

    [0xAF,0x00,0x00], [0xAF,0x00,0x5F], [0xAF,0x00,0x87], [0xAF,0x00,0xAF], [0xAF,0x00,0xD7], [0xAF,0x00,0xFF],
    [0xAF,0x5F,0x00], [0xAF,0x5F,0x5F], [0xAF,0x5F,0x87], [0xAF,0x5F,0xAF], [0xAF,0x5F,0xD7], [0xAF,0x5F,0xFF],
    [0xAF,0x87,0x00], [0xAF,0x87,0x5F], [0xAF,0x87,0x87], [0xAF,0x87,0xAF], [0xAF,0x87,0xD7], [0xAF,0x87,0xFF],
    [0xAF,0xAF,0x00], [0xAF,0xAF,0x5F], [0xAF,0xAF,0x87], [0xAF,0xAF,0xAF], [0xAF,0xAF,0xD7], [0xAF,0xAF,0xFF],
    [0xAF,0xD7,0x00], [0xAF,0xD7,0x5F], [0xAF,0xD7,0x87], [0xAF,0xD7,0xAF], [0xAF,0xD7,0xD7], [0xAF,0xD7,0xFF],
    [0xAF,0xFF,0x00], [0xAF,0xFF,0x5F], [0xAF,0xFF,0x87], [0xAF,0xFF,0xAF], [0xAF,0xFF,0xD7], [0xAF,0xFF,0xFF],

    [0xD7,0x00,0x00], [0xD7,0x00,0x5F], [0xD7,0x00,0x87], [0xD7,0x00,0xAF], [0xD7,0x00,0xD7], [0xD7,0x00,0xFF],
    [0xD7,0x5F,0x00], [0xD7,0x5F,0x5F], [0xD7,0x5F,0x87], [0xD7,0x5F,0xAF], [0xD7,0x5F,0xD7], [0xD7,0x5F,0xFF],
    [0xD7,0x87,0x00], [0xD7,0x87,0x5F], [0xD7,0x87,0x87], [0xD7,0x87,0xAF], [0xD7,0x87,0xD7], [0xD7,0x87,0xFF],
    [0xD7,0xAF,0x00], [0xD7,0xAF,0x5F], [0xD7,0xAF,0x87], [0xD7,0xAF,0xAF], [0xD7,0xAF,0xD7], [0xD7,0xAF,0xFF],
    [0xD7,0xD7,0x00], [0xD7,0xD7,0x5F], [0xD7,0xD7,0x87], [0xD7,0xD7,0xAF], [0xD7,0xD7,0xD7], [0xD7,0xD7,0xFF],
    [0xD7,0xFF,0x00], [0xD7,0xFF,0x5F], [0xD7,0xFF,0x87], [0xD7,0xFF,0xAF], [0xD7,0xFF,0xD7], [0xD7,0xFF,0xFF],

    [0xFF,0x00,0x00], [0xFF,0x00,0x5F], [0xFF,0x00,0x87], [0xFF,0x00,0xAF], [0xFF,0x00,0xD7], [0xFF,0x00,0xFF],
    [0xFF,0x5F,0x00], [0xFF,0x5F,0x5F], [0xFF,0x5F,0x87], [0xFF,0x5F,0xAF], [0xFF,0x5F,0xD7], [0xFF,0x5F,0xFF],
    [0xFF,0x87,0x00], [0xFF,0x87,0x5F], [0xFF,0x87,0x87], [0xFF,0x87,0xAF], [0xFF,0x87,0xD7], [0xFF,0x87,0xFF],
    [0xFF,0xAF,0x00], [0xFF,0xAF,0x5F], [0xFF,0xAF,0x87], [0xFF,0xAF,0xAF], [0xFF,0xAF,0xD7], [0xFF,0xAF,0xFF],
    [0xFF,0xD7,0x00], [0xFF,0xD7,0x5F], [0xFF,0xD7,0x87], [0xFF,0xD7,0xAF], [0xFF,0xD7,0xD7], [0xFF,0xD7,0xFF],
    [0xFF,0xFF,0x00], [0xFF,0xFF,0x5F], [0xFF,0xFF,0x87], [0xFF,0xFF,0xAF], [0xFF,0xFF,0xD7], [0xFF,0xFF,0xFF],

    // extra grayscale
    [0x08,0x08,0x08], [0x12,0x12,0x12], [0x1C,0x1C,0x1C], [0x26,0x26,0x26], [0x30,0x30,0x30], [0x3A,0x3A,0x3A],
    [0x44,0x44,0x44], [0x4E,0x4E,0x4E], [0x58,0x58,0x58], [0x62,0x62,0x62], [0x6C,0x6C,0x6C], [0x76,0x76,0x76],
    [0x80,0x80,0x80], [0x8A,0x8A,0x8A], [0x94,0x94,0x94], [0x9E,0x9E,0x9E], [0xA8,0xA8,0xA8], [0xB2,0xB2,0xB2],
    [0xBC,0xBC,0xBC], [0xC6,0xC6,0xC6], [0xD0,0xD0,0xD0], [0xDA,0xDA,0xDA], [0xE4,0xE4,0xE4], [0xEE,0xEE,0xEE],
];

/// Closest 256 color to a given grayscale value
// thanks to [ansi_colours](https://crates.io/crates/ansi_colours)
#[rustfmt::skip]
pub static GRAY_TO_256: [u8; 256] = [
        16,  16,  16,  16,  16, 232, 232, 232,
        232, 232, 232, 232, 232, 232, 233, 233,
        233, 233, 233, 233, 233, 233, 233, 233,
        234, 234, 234, 234, 234, 234, 234, 234,
        234, 234, 235, 235, 235, 235, 235, 235,
        235, 235, 235, 235, 236, 236, 236, 236,
        236, 236, 236, 236, 236, 236, 237, 237,
        237, 237, 237, 237, 237, 237, 237, 237,
        238, 238, 238, 238, 238, 238, 238, 238,
        238, 238, 239, 239, 239, 239, 239, 239,
        239, 239, 239, 239, 240, 240, 240, 240,
        240, 240, 240, 240,  59,  59,  59,  59,
        59,  241, 241, 241, 241, 241, 241, 241,
        242, 242, 242, 242, 242, 242, 242, 242,
        242, 242, 243, 243, 243, 243, 243, 243,
        243, 243, 243, 244, 244, 244, 244, 244,
        244, 244, 244, 244, 102, 102, 102, 102,
        102, 245, 245, 245, 245, 245, 245, 246,
        246, 246, 246, 246, 246, 246, 246, 246,
        246, 247, 247, 247, 247, 247, 247, 247,
        247, 247, 247, 248, 248, 248, 248, 248,
        248, 248, 248, 248, 145, 145, 145, 145,
        145, 249, 249, 249, 249, 249, 249, 250,
        250, 250, 250, 250, 250, 250, 250, 250,
        250, 251, 251, 251, 251, 251, 251, 251,
        251, 251, 251, 252, 252, 252, 252, 252,
        252, 252, 252, 252, 188, 188, 188, 188,
        188, 253, 253, 253, 253, 253, 253, 254,
        254, 254, 254, 254, 254, 254, 254, 254,
        254, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 231,
        231, 231, 231, 231, 231, 231, 231, 231,
];

#[rustfmt::skip]
static FMT_U8: [&'static [u8]; 256] = [
    b"0", b"1",  b"2", b"3", b"4", b"5", b"6", b"7", b"8", b"9",
    b"10", b"11",  b"12", b"13", b"14", b"15", b"16", b"17", b"18", b"19",
    b"20", b"21",  b"22", b"23", b"24", b"25", b"26", b"27", b"28", b"29",
    b"30", b"31",  b"32", b"33", b"34", b"35", b"36", b"37", b"38", b"39",
    b"40", b"41",  b"42", b"43", b"44", b"45", b"46", b"47", b"48", b"49",
    b"50", b"51",  b"52", b"53", b"54", b"55", b"56", b"57", b"58", b"59",
    b"60", b"61",  b"62", b"63", b"64", b"65", b"66", b"67", b"68", b"69",
    b"70", b"71",  b"72", b"73", b"74", b"75", b"76", b"77", b"78", b"79",
    b"80", b"81",  b"82", b"83", b"84", b"85", b"86", b"87", b"88", b"89",
    b"90", b"91",  b"92", b"93", b"94", b"95", b"96", b"97", b"98", b"99",

    b"100", b"101",  b"102", b"103", b"104", b"105", b"106", b"107", b"108", b"109",
    b"110", b"111",  b"112", b"113", b"114", b"115", b"116", b"117", b"118", b"119",
    b"120", b"121",  b"122", b"123", b"124", b"125", b"126", b"127", b"128", b"129",
    b"130", b"131",  b"132", b"133", b"134", b"135", b"136", b"137", b"138", b"139",
    b"140", b"141",  b"142", b"143", b"144", b"145", b"146", b"147", b"148", b"149",
    b"150", b"151",  b"152", b"153", b"154", b"155", b"156", b"157", b"158", b"159",
    b"160", b"161",  b"162", b"163", b"164", b"165", b"166", b"167", b"168", b"169",
    b"170", b"171",  b"172", b"173", b"174", b"175", b"176", b"177", b"178", b"179",
    b"180", b"181",  b"182", b"183", b"184", b"185", b"186", b"187", b"188", b"189",
    b"190", b"191",  b"192", b"193", b"194", b"195", b"196", b"197", b"198", b"199",

    b"200", b"201",  b"202", b"203", b"204", b"205", b"206", b"207", b"208", b"209",
    b"210", b"211",  b"212", b"213", b"214", b"215", b"216", b"217", b"218", b"219",
    b"220", b"221",  b"222", b"223", b"224", b"225", b"226", b"227", b"228", b"229",
    b"230", b"231",  b"232", b"233", b"234", b"235", b"236", b"237", b"238", b"239",
    b"240", b"241",  b"242", b"243", b"244", b"245", b"246", b"247", b"248", b"249",
    b"250", b"251",  b"252", b"253", b"254", b"255"
];

pub struct Fg24<'a>(core::iter::Flatten<core::array::IntoIter<&'a [u8], 7>>);
impl<'a> Iterator for Fg24<'a> {
    type Item = &'a u8;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> { self.0.next() }
}
impl<'a> From<[u8; 3]> for Fg24<'a> {
    #[inline(always)]
    fn from([r, g, b]: [u8; 3]) -> Self {
        Fg24(
            [
                b"\x1B[38;2;",
                FMT_U8[r as usize],
                b";",
                FMT_U8[g as usize],
                b";",
                FMT_U8[b as usize],
                b"m",
            ]
            .into_iter()
            .flatten(),
        )
    }
}
pub struct Bg24<'a>(core::iter::Flatten<core::array::IntoIter<&'a [u8], 7>>);
impl<'a> Iterator for Bg24<'a> {
    type Item = &'a u8;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> { self.0.next() }
}
impl<'a> From<[u8; 3]> for Bg24<'a> {
    #[inline(always)]
    fn from([r, g, b]: [u8; 3]) -> Self {
        Bg24(
            [
                b"\x1B[48;2;",
                FMT_U8[r as usize],
                b";",
                FMT_U8[g as usize],
                b";",
                FMT_U8[b as usize],
                b"m",
            ]
            .into_iter()
            .flatten(),
        )
    }
}
pub struct Fg8<'a>(core::iter::Flatten<core::array::IntoIter<&'a [u8], 3>>);
impl<'a> Iterator for Fg8<'a> {
    type Item = &'a u8;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> { self.0.next() }
}
impl<'a> From<u8> for Fg8<'a> {
    #[inline(always)]
    fn from(p: u8) -> Self { Fg8([b"\x1B[38;5;", FMT_U8[p as usize], b"m"].into_iter().flatten()) }
}
pub struct Bg8<'a>(core::iter::Flatten<core::array::IntoIter<&'a [u8], 3>>);
impl<'a> Iterator for Bg8<'a> {
    type Item = &'a u8;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> { self.0.next() }
}
impl<'a> From<u8> for Bg8<'a> {
    #[inline(always)]
    fn from(p: u8) -> Self { Bg8([b"\x1B[48;5;", FMT_U8[p as usize], b"m"].into_iter().flatten()) }
}

/// Base trait for converting a raw pixel value into an ansi color.
pub trait RawAnsiPixel: Sized {
    const CHANNELS: usize;
    type Repr: Clone + Copy + Send + Sync + Sized;
    fn ansi_color_24bit(p: <Self as RawAnsiPixel>::Repr, a: &ColorAttributes) -> [u8; 3];
    fn ansi_gray_24bit(p: <Self as RawAnsiPixel>::Repr, a: &ColorAttributes) -> [u8; 3];
    fn ansi_color_8bit(p: <Self as RawAnsiPixel>::Repr, a: &ColorAttributes) -> u8;
    fn ansi_gray_8bit(p: <Self as RawAnsiPixel>::Repr, a: &ColorAttributes) -> u8;
}

impl RawAnsiPixel for Rgb<u8> {
    const CHANNELS: usize = 3;
    type Repr = [u8; 3];
    #[inline(always)]
    fn ansi_color_24bit(p: <Self as RawAnsiPixel>::Repr, _a: &ColorAttributes) -> [u8; 3] { p }
    #[inline]
    fn ansi_gray_24bit(p: <Self as RawAnsiPixel>::Repr, _a: &ColorAttributes) -> [u8; 3] {
        let v = luma(p);
        [v, v, v]
    }
    #[inline(always)]
    fn ansi_color_8bit(p: <Self as RawAnsiPixel>::Repr, a: &ColorAttributes) -> u8 { rgb_to_256(p, a) }
    #[inline(always)]
    fn ansi_gray_8bit(p: <Self as RawAnsiPixel>::Repr, _a: &ColorAttributes) -> u8 { GRAY_TO_256[luma(p) as usize] }
}

impl RawAnsiPixel for Luma<u8> {
    const CHANNELS: usize = 1;
    type Repr = u8;
    #[inline(always)]
    fn ansi_color_24bit(p: <Self as RawAnsiPixel>::Repr, a: &ColorAttributes) -> [u8; 3] { Self::ansi_gray_24bit(p, a) }
    #[inline(always)]
    fn ansi_gray_24bit(p: <Self as RawAnsiPixel>::Repr, _: &ColorAttributes) -> [u8; 3] { [p, p, p] }
    #[inline(always)]
    fn ansi_color_8bit(p: <Self as RawAnsiPixel>::Repr, a: &ColorAttributes) -> u8 { Self::ansi_gray_8bit(p, a) }
    #[inline(always)]
    fn ansi_gray_8bit(p: <Self as RawAnsiPixel>::Repr, _: &ColorAttributes) -> u8 { GRAY_TO_256[p as usize] }
}

pub const RESERVE_24: usize = 39;
pub const RESERVE_8: usize = 23;
pub trait AnsiColor {
    const RESERVE_SIZE: usize;
    type Fg<'a>: Iterator<Item = &'a u8> + From<<Self as AnsiColor>::Repr> + 'a;
    type Bg<'a>: Iterator<Item = &'a u8> + From<<Self as AnsiColor>::Repr> + 'a;
    type Repr: Copy + Clone + Send + Sync + Sized;
    fn to_repr<P: RawAnsiPixel>(p: <P as RawAnsiPixel>::Repr, a: &ColorAttributes) -> <Self as AnsiColor>::Repr;
    #[inline(always)]
    fn fg<'a, P: RawAnsiPixel>(p: <P as RawAnsiPixel>::Repr, a: &ColorAttributes) -> <Self as AnsiColor>::Fg<'a> {
        <Self as AnsiColor>::Fg::from(Self::to_repr::<P>(p, a))
    }
    #[inline(always)]
    fn bg<'a, P: RawAnsiPixel>(p: <P as RawAnsiPixel>::Repr, a: &ColorAttributes) -> <Self as AnsiColor>::Bg<'a> {
        <Self as AnsiColor>::Bg::from(Self::to_repr::<P>(p, a))
    }
}
pub struct Color24;
impl AnsiColor for Color24 {
    const RESERVE_SIZE: usize = RESERVE_24;
    type Fg<'a> = Fg24<'a>;
    type Bg<'a> = Bg24<'a>;
    type Repr = [u8; 3];
    #[inline(always)]
    fn to_repr<P: RawAnsiPixel>(p: <P as RawAnsiPixel>::Repr, a: &ColorAttributes) -> <Self as AnsiColor>::Repr {
        P::ansi_color_24bit(p, a)
    }
}
pub struct Color8;
impl AnsiColor for Color8 {
    const RESERVE_SIZE: usize = RESERVE_8;
    type Fg<'a> = Fg8<'a>;
    type Bg<'a> = Bg8<'a>;
    type Repr = u8;
    #[inline(always)]
    fn to_repr<P: RawAnsiPixel>(p: <P as RawAnsiPixel>::Repr, a: &ColorAttributes) -> <Self as AnsiColor>::Repr {
        P::ansi_color_8bit(p, a)
    }
}
pub struct Gray24;
impl AnsiColor for Gray24 {
    const RESERVE_SIZE: usize = RESERVE_24;
    type Fg<'a> = Fg24<'a>;
    type Bg<'a> = Bg24<'a>;
    type Repr = [u8; 3];
    #[inline(always)]
    fn to_repr<P: RawAnsiPixel>(p: <P as RawAnsiPixel>::Repr, a: &ColorAttributes) -> <Self as AnsiColor>::Repr {
        P::ansi_gray_24bit(p, a)
    }
}
pub struct Gray8;
impl AnsiColor for Gray8 {
    const RESERVE_SIZE: usize = RESERVE_8;
    type Fg<'a> = Fg8<'a>;
    type Bg<'a> = Bg8<'a>;
    type Repr = u8;
    #[inline(always)]
    fn to_repr<P: RawAnsiPixel>(p: <P as RawAnsiPixel>::Repr, a: &ColorAttributes) -> <Self as AnsiColor>::Repr {
        P::ansi_gray_8bit(p, a)
    }
}

pub struct PixelConverter;
impl PixelConverter {
    #[inline]
    pub fn fg<'a, P: RawAnsiPixel, C: AnsiColor>(
        p: <P as RawAnsiPixel>::Repr,
        a: &ColorAttributes,
    ) -> impl Iterator<Item = &'a u8> {
        C::fg::<P>(p, a)
    }
    #[inline]
    pub fn bg<'a, P: RawAnsiPixel, C: AnsiColor>(
        p: <P as RawAnsiPixel>::Repr,
        a: &ColorAttributes,
    ) -> impl Iterator<Item = &'a u8> {
        C::bg::<P>(p, a)
    }
}
