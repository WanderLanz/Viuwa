use std::iter;

use image::{buffer::Rows, DynamicImage, GrayImage, Pixel, RgbImage};

use crate::UPPER_HALF_BLOCK;

use super::{
        ansi::{self, color::AnsiPixel, cursor},
        ColorAttributes, ColorType,
};

pub enum ViuwaImageBuffer {
        Rgb(RgbImage),
        Gray(GrayImage),
}

impl ViuwaImageBuffer {
        pub fn from(img: DynamicImage, color_type: ColorType) -> Self {
                use ViuwaImageBuffer::*;
                if img.color().has_color() {
                        if let ColorType::Gray | ColorType::Gray256 = color_type {
                                Gray(img.into_luma8())
                        } else {
                                Rgb(img.into_rgb8())
                        }
                } else {
                        Gray(img.into_luma8())
                }
        }
        pub fn dimensions(&self) -> (u32, u32) {
                use ViuwaImageBuffer::*;
                match self {
                        Rgb(ref img) => img.dimensions(),
                        Gray(ref img) => img.dimensions(),
                }
        }
}

pub struct ViuwaImage {
        buf: ViuwaImageBuffer,
        color_type: ColorType,
        color_attrs: ColorAttributes,
}

impl ViuwaImage {
        pub fn new(orig: DynamicImage, color_type: ColorType, color_attrs: ColorAttributes) -> Self {
                Self {
                        buf: ViuwaImageBuffer::from(orig, color_type),
                        color_type,
                        color_attrs,
                }
        }
        /// Returns the image as ansi lines, with cursor movement and ansi resets to center within given window
        pub fn to_ansi_windowed(&self, window_size: (u16, u16)) -> Vec<String> {
                let (w, h) = self.buf.dimensions();
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
        /// Returns the image as ansi lines, with newlines and ansi resets
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
        /// Covert rows of an image to 24-bit or 8-bit ANSI color escapes and "▀"and return them as a vector of strings, 2 rows of pixels per row of ansi
        ///
        /// No cursor movement or ANSI resets
        ///
        /// Bulk of the work is done here
        pub fn to_ansi_rows_raw(&self) -> Vec<String> {
                match self.buf {
                        // image are already set to Rgb or Luma based on the format
                        ViuwaImageBuffer::Rgb(ref img) => {
                                let w = img.width() as u16;
                                let rows = img.rows();
                                match self.color_type {
                                        ColorType::Color => rows_to_ansi_24b(w, &self.color_attrs, rows),
                                        ColorType::Color256 => rows_to_ansi_8b(w, &self.color_attrs, rows),
                                        _ => unreachable!(
                                        "ViuwaImageBuffer::Rgb with gray ColorType, one of the developers did an oopsie!"
                                ),
                                }
                        }
                        ViuwaImageBuffer::Gray(ref img) => {
                                let w = img.width() as u16;
                                let rows = img.rows();
                                match self.color_type {
                                        ColorType::Color256 | ColorType::Gray256 => {
                                                rows_to_ansi_8b(w, &self.color_attrs, rows)
                                        }
                                        _ => rows_to_ansi_24b(w, &self.color_attrs, rows),
                                }
                        }
                }
        }
}

pub trait AnsiImage {
        /// Returns the image as ansi lines, with cursor movement and ansi resets to center within given window
        fn into_ansi_windowed(
                self,
                color_type: ColorType,
                color_attrs: ColorAttributes,
                window_size: (u16, u16),
        ) -> Vec<String>;
        /// Returns the image as ansi lines, with newlines and ansi resets
        fn into_ansi_inline(self, color_type: ColorType, color_attrs: ColorAttributes) -> Vec<String>;
        /// Covert rows of an image to 24-bit or 8-bit ANSI color escapes and "▀"and return them as a vector of strings, 2 rows of pixels per row of ansi
        ///
        /// No cursor movement or ANSI resets
        ///
        /// Bulk of the work is done here
        fn into_ansi_raw(self, color_type: ColorType, color_attrs: ColorAttributes) -> Vec<String>;
}

impl AnsiImage for DynamicImage {
        #[inline]
        fn into_ansi_windowed(
                self,
                color_type: ColorType,
                color_attrs: ColorAttributes,
                window_size: (u16, u16),
        ) -> Vec<String> {
                ViuwaImage::new(self, color_type, color_attrs).to_ansi_windowed(window_size)
        }
        #[inline]
        fn into_ansi_inline(self, color_type: ColorType, color_attrs: ColorAttributes) -> Vec<String> {
                ViuwaImage::new(self, color_type, color_attrs).to_ansi_inline()
        }
        #[inline]
        fn into_ansi_raw(self, color_type: ColorType, color_attrs: ColorAttributes) -> Vec<String> {
                ViuwaImage::new(self, color_type, color_attrs).to_ansi_rows_raw()
        }
}

/// Map rows of an image to 24-bit ANSI pixels and return them as a vector of strings, 2 rows of pixels per row of ansi
fn rows_to_ansi_24b<P>(w: u16, attrs: &ColorAttributes, mut rows: Rows<P>) -> Vec<String>
where
        P: Pixel<Subpixel = u8> + AnsiPixel,
{
        iter::repeat_with(move || (rows.next(), rows.next()))
                .map_while(|pxs| match pxs {
                        (Some(fgs), Some(bgs)) => {
                                Some(fgs.zip(bgs).fold(String::with_capacity(w as usize * 39), |mut a, (fg, bg)| {
                                        a.push_str(&fg.fg_24b(attrs));
                                        a.push_str(&bg.bg_24b(attrs));
                                        a.push_str(UPPER_HALF_BLOCK);
                                        a
                                }))
                        }
                        (Some(fgs), None) => Some(fgs.fold(String::with_capacity(w as usize * 20), |mut a, fg| {
                                a.push_str(&fg.fg_24b(attrs));
                                a.push_str(UPPER_HALF_BLOCK);
                                a
                        })),
                        _ => None,
                })
                .collect()
}
/// Map rows of an image to 8-bit ANSI pixels and return them as a vector of strings, 2 rows of pixels per row of ansi
fn rows_to_ansi_8b<P>(w: u16, attrs: &ColorAttributes, mut rows: Rows<P>) -> Vec<String>
where
        P: Pixel<Subpixel = u8> + AnsiPixel,
{
        iter::repeat_with(move || (rows.next(), rows.next()))
                .map_while(|pxs| match pxs {
                        (Some(fgs), Some(bgs)) => {
                                Some(fgs.zip(bgs).fold(String::with_capacity(w as usize * 23), |mut a, (fg, bg)| {
                                        a.push_str(&fg.fg_8b(attrs));
                                        a.push_str(&bg.bg_8b(attrs));
                                        a.push_str(UPPER_HALF_BLOCK);
                                        a
                                }))
                        }
                        (Some(fgs), None) => Some(fgs.fold(String::with_capacity(w as usize * 12), |mut a, fg| {
                                a.push_str(&fg.fg_8b(attrs));
                                a.push_str(UPPER_HALF_BLOCK);
                                a
                        })),
                        _ => None,
                })
                .collect()
}
