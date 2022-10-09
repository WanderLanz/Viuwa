use std::iter;

use image::{
        buffer::{Pixels, Rows},
        imageops::FilterType,
        DynamicImage, GrayImage, Luma, Pixel, Rgb, RgbImage,
};

use crate::UPPER_HALF_BLOCK;

use super::{
        ansi::{self, color, cursor},
        OutFormat,
};

pub enum ViuwaImage {
        Rgb(RgbImage, OutFormat),
        Luma(GrayImage, OutFormat),
}

impl ViuwaImage {
        pub fn new(img: DynamicImage, format: OutFormat) -> Self {
                match img.color() {
                        image::ColorType::L8 => ViuwaImage::Luma(img.into_luma8(), format),
                        image::ColorType::Rgb8 => match format {
                                OutFormat::AnsiGrey => ViuwaImage::Luma(img.into_luma8(), format),
                                _ => ViuwaImage::Rgb(img.into_rgb8(), format),
                        },
                        _ => unreachable!("Failed to convert image to RGB or Luma before creating ViuwaImage"),
                }
        }
        pub fn dimensions(&self) -> (u32, u32) {
                match self {
                        ViuwaImage::Rgb(img, _) => img.dimensions(),
                        ViuwaImage::Luma(img, _) => img.dimensions(),
                }
        }
        pub fn width(&self) -> u32 {
                match self {
                        ViuwaImage::Rgb(img, _) => img.width(),
                        ViuwaImage::Luma(img, _) => img.width(),
                }
        }
        pub fn height(&self) -> u32 {
                match self {
                        ViuwaImage::Rgb(img, _) => img.height(),
                        ViuwaImage::Luma(img, _) => img.height(),
                }
        }
        /// Returns the image as ansi lines, with cursor movement and ansi resets to center within given window.
        pub fn to_ansi_window(&self, window_size: (u16, u16)) -> Vec<String> {
                let (w, h) = self.dimensions();
                let ox = ((window_size.0 as u32 - w) / 2) as u16;
                let oy = (((window_size.1 as u32 * 2) - h) / 4) as u16;
                self.to_ansi_rows_raw()
                        .into_iter()
                        .enumerate()
                        .map(|(y, s)| {
                                let mut ns = cursor::to(ox, oy + y as u16);
                                ns.push_str(&s);
                                ns.push_str(ansi::attr::RESET);
                                ns
                        })
                        .collect()
        }
        /// Returns the image as ansi lines, with newline ansi and ansi resets.
        pub fn to_ansi_inline(&self) -> Vec<String> {
                self.to_ansi_rows_raw()
                        .into_iter()
                        .map(|mut s| {
                                s.push_str(ansi::attr::RESET);
                                s.push('\n');
                                s
                        })
                        .collect()
        }
        /// Covert rows of an image to ANSI color and "▀"and return them as a vector of strings, 2 rows of pixels per row of ansi.
        ///
        /// No cursor movement or ANSI reset is done.
        pub fn to_ansi_rows_raw(&self) -> Vec<String> {
                match self {
                        // image are already set to Rgb or Luma based on the format
                        ViuwaImage::Rgb(img, fmt) => {
                                match fmt {
                                        OutFormat::AnsiRgb => Self::map_rows(
                                                img.width() as u16,
                                                img.rows(),
                                                Self::_rows2_24rgb,
                                                Self::_row_24rgb,
                                        ),
                                        OutFormat::Ansi256 => Self::map_rows(
                                                img.width() as u16,
                                                img.rows(),
                                                Self::_row2_8rgb,
                                                Self::_row_8rgb,
                                        ),
                                        _ => unreachable!(
                                                "Failed to format image before attempting to transform into ansi."
                                        ), // ignore sixel, iterm, and greyscale
                                }
                        }
                        ViuwaImage::Luma(img, fmt) => {
                                match fmt {
                                        OutFormat::AnsiGrey => Self::map_rows(
                                                img.width() as u16,
                                                img.rows(),
                                                Self::_row2_24grey,
                                                Self::_row_24grey,
                                        ),
                                        OutFormat::Ansi256 => Self::map_rows(
                                                img.width() as u16,
                                                img.rows(),
                                                Self::_row2_8grey,
                                                Self::_row_8grey,
                                        ),
                                        _ => unreachable!(
                                                "Failed to format image before attempting to transform into ansi."
                                        ), // ignore sixel, iterm, and color
                                }
                        }
                }
        }
        /// Map rows of an image to ANSI escape sequences and return them as a vector of strings, 2 rows of pixels per row of ansi.
        fn map_rows<P, F2, F1>(w: u16, mut rows: Rows<P>, f2: F2, f1: F1) -> Vec<String>
        where
                P: Pixel<Subpixel = u8>,
                F2: Fn(u16, Pixels<P>, Pixels<P>) -> String,
                F1: Fn(u16, Pixels<P>) -> String,
        {
                iter::repeat_with(move || (rows.next(), rows.next()))
                        .map_while(|(fgs, bgs)| match (fgs, bgs) {
                                (Some(fgs), Some(bgs)) => Some(f2(w, fgs, bgs)),
                                (Some(fgs), None) => Some(f1(w, fgs)),
                                _ => None,
                        })
                        .collect()
        }
        /// 2 rows to 24bit color
        fn _rows2_24rgb(w: u16, fgs: Pixels<Rgb<u8>>, bgs: Pixels<Rgb<u8>>) -> String {
                Self::__row2_24bit(w, fgs, bgs, |mut a, (fg, bg)| {
                        a.push_str(&color::set_fg24_color(fg.0));
                        a.push_str(&color::set_bg24_color(bg.0));
                        a.push_str(UPPER_HALF_BLOCK);
                        a
                })
        }
        /// 2 rows to 24bit greyscale
        fn _row2_24grey(w: u16, fgs: Pixels<Luma<u8>>, bgs: Pixels<Luma<u8>>) -> String {
                Self::__row2_24bit(w, fgs, bgs, |mut a, (fg, bg)| {
                        a.push_str(&color::set_fg24_grey(fg.0[0]));
                        a.push_str(&color::set_bg24_grey(bg.0[0]));
                        a.push_str(UPPER_HALF_BLOCK);
                        a
                })
        }
        /// 2 rows to 8bit color
        fn _row2_8rgb(w: u16, fgs: Pixels<Rgb<u8>>, bgs: Pixels<Rgb<u8>>) -> String {
                Self::__row2_8bit(w, fgs, bgs, |mut a, (fg, bg)| {
                        a.push_str(&color::set_fg8(color::rgb_to_256(fg.0)));
                        a.push_str(&color::set_bg8(color::rgb_to_256(bg.0)));
                        a.push_str(UPPER_HALF_BLOCK);
                        a
                })
        }
        /// 2 rows to 8bit greyscale
        fn _row2_8grey(w: u16, fgs: Pixels<Luma<u8>>, bgs: Pixels<Luma<u8>>) -> String {
                Self::__row2_8bit(w, fgs, bgs, |mut a, (fg, bg)| {
                        a.push_str(&color::set_fg8(color::GREY_TO_256[fg.0[0] as usize]));
                        a.push_str(&color::set_bg8(color::GREY_TO_256[bg.0[0] as usize]));
                        a.push_str(UPPER_HALF_BLOCK);
                        a
                })
        }
        /// 1 row to 24bit color
        fn _row_24rgb(w: u16, fgs: Pixels<Rgb<u8>>) -> String {
                Self::__row_24bit(w, fgs, |mut a, fg| {
                        a.push_str(&color::set_fg24_color(fg.0));
                        a.push_str(UPPER_HALF_BLOCK);
                        a
                })
        }
        /// 1 row to 24bit greyscale
        fn _row_24grey(w: u16, fgs: Pixels<Luma<u8>>) -> String {
                Self::__row_24bit(w, fgs, |mut a, fg| {
                        a.push_str(&color::set_fg24_grey(fg.0[0]));
                        a.push_str(UPPER_HALF_BLOCK);
                        a
                })
        }
        /// 1 row to 8bit color
        fn _row_8rgb(w: u16, fgs: Pixels<Rgb<u8>>) -> String {
                Self::__row_8bit(w, fgs, |mut a, fg| {
                        a.push_str(&color::set_fg8(color::rgb_to_256(fg.0)));
                        a.push_str(UPPER_HALF_BLOCK);
                        a
                })
        }
        /// 1 row to 8bit greyscale
        fn _row_8grey(w: u16, fgs: Pixels<Luma<u8>>) -> String {
                Self::__row_8bit(w, fgs, |mut a, fg| {
                        a.push_str(&color::set_fg8(color::GREY_TO_256[fg.0[0] as usize]));
                        a.push_str(UPPER_HALF_BLOCK);
                        a
                })
        }
        /// Convert 2 rows of pixels to a string of 24-bit-color ANSI escape codes
        fn __row2_24bit<P, F>(w: u16, fgs: Pixels<P>, bgs: Pixels<P>, f: F) -> String
        where
                P: Pixel<Subpixel = u8>,
                F: FnMut(String, (&P, &P)) -> String,
        {
                fgs.zip(bgs).fold(String::with_capacity(w as usize * 39), f)
        }
        /// Convert 2 rows of pixels to a string of 8-bit-color ANSI escape codes
        fn __row2_8bit<P, F>(w: u16, fgs: Pixels<P>, bgs: Pixels<P>, f: F) -> String
        where
                P: Pixel<Subpixel = u8>,
                F: FnMut(String, (&P, &P)) -> String,
        {
                fgs.zip(bgs).fold(String::with_capacity(w as usize * 23), f)
        }
        /// Convert a row of pixels to a string of 24-bit-color ANSI escape codes
        fn __row_24bit<P, F>(w: u16, fgs: Pixels<P>, f: F) -> String
        where
                P: Pixel<Subpixel = u8>,
                F: FnMut(String, &P) -> String,
        {
                fgs.fold(String::with_capacity(w as usize * 20), f)
        }
        /// Convert a row of pixels to a string of 8-bit-color ANSI escape codes
        fn __row_8bit<P, F>(w: u16, fgs: Pixels<P>, f: F) -> String
        where
                P: Pixel<Subpixel = u8>,
                F: FnMut(String, &P) -> String,
        {
                fgs.fold(String::with_capacity(w as usize * 12), f)
        }
}
