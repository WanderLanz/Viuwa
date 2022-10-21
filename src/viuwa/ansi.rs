//! trying my best to make an ANSI module for all platforms...
//!
//! refs:
//!  - vt100:                  
//!     - https://vt100.net/docs/vt100-ug/contents.html
//!  - fnky's gist:        
//!     - https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
//!  - xterm:              
//!     - https://www.xfree86.org/current/ctlseqs.html
//!     - https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
//!  - windows:            
//!     - https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences
//!  - linux:              
//!     - https://en.wikipedia.org/wiki/ANSI_escape_code
//!  - iterm:              
//!     - https://iterm2.com/documentation-escape-codes.html
//!     - https://chromium.googlesource.com/apps/libapps/+/master/hterm/doc/ControlSequences.md#OSC-1337
//!  - kitty:              
//!     - https://sw.kovidgoyal.net/kitty/graphics-protocol.html
//!  - alacritty:          
//!     - https://github.com/alacritty/alacritty/blob/master/docs/escape_support.md
//!  - mintty:             
//!     - https://github.com/mintty/mintty/wiki/CtrlSeqs
//!  - sixel:              
//!     - https://en.wikipedia.org/wiki/Sixel
//!     - https://konfou.xyz/posts/sixel-for-terminal-graphics
//!  - sixel spec:          
//!     - https://vt100.net/docs/vt510-rm/sixel.html
//!  
//! for reference:
//!  - ESC = escape
//!  - ST = string terminator
//!  - CSI = control sequence introducer
//!  - OSC = operating system command
//!  - DCS = device control string
//!  - APM = application program mode
//!  - SGR = select graphic rendition
#![allow(dead_code)]
use image::{DynamicImage, GenericImageView, Pixel};

use crate::BoxResult;

#[cfg(target_family = "wasm")]
use std::io::{stdin, Read};
use std::{fmt, io};

use self::color::AnsiPixel;

#[cfg(feature = "rayon-converter")]
use rayon::prelude::*;

use super::{ColorAttributes, ColorType};
macro_rules! esc {
        ($( $l:expr ),*) => { concat!('\x1B', $( $l ),*) };
}
macro_rules! csi {
        ($( $l:expr ),*) => { concat!(esc!("["), $( $l ),*) };
}
macro_rules! osc {
        ($( $l:expr ),*) => { concat!(esc!("]"), $( $l ),*) };
}
// macro_rules! dcs {
//         ($( $l:expr ),*) => { concat!(esc!("P"), $( $l ),*) };
// }
// macro_rules! apm {
//         ($( $l:expr ),*) => { concat!(esc!("_"), $( $l ),*) };
// }
#[macro_export]
macro_rules! st {
        ($( $l:expr ),*) => { concat!($( $l ),*, esc!("\\")) };
}

const ESC: &str = esc!();
const CSI: &str = esc!("[");
const OSC: &str = esc!("]");
const DCS: &str = esc!("P");
const APM: &str = esc!("_");
const ST: &str = esc!("\\");

#[derive(Debug, Clone, Copy)]
struct Coord {
        x: u16,
        y: u16,
}
impl Coord {
        /// Safe maximum of Coord assuming 16-bit signed integers is used by terminal for cursor position. (Max value is 32767 for c_short)
        const MAX: Coord = Coord {
                x: 0x7FFF,
                y: 0x7FFF,
        };
        // fn new(x: u16, y: u16) -> Self { Self { x, y } }
        // fn as_report(&self) -> String { format!("{};{}", self.y, self.x) }
        /// e.g. "1;2" -> Coord { x: 2, y: 1 }
        fn try_from_report(s: &str) -> BoxResult<Self> {
                let mut iter = s.split(';');
                if let (Some(y), Some(x)) = (iter.next(), iter.next()) {
                        Ok(Self {
                                x: x.parse()?,
                                y: y.parse()?,
                        })
                } else {
                        Err("invalid coord".into())
                }
        }
}

pub mod attr {
        pub const RESET: &str = csi!("0m");
        //...
}

pub mod color;

#[derive(Debug, Clone)]
pub struct AnsiLineBuffer(pub String);
impl AnsiLineBuffer {
        pub const RESERVE_SIZE_24B: usize = 39;
        pub const RESERVE_SIZE_8B: usize = 23;
        #[inline]
        pub fn reserve_24b(&mut self, width: u16) {
                self.0.reserve(width as usize * Self::RESERVE_SIZE_24B);
        }
        #[inline]
        pub fn reserve_8b(&mut self, width: u16) {
                self.0.reserve(width as usize * Self::RESERVE_SIZE_8B);
        }
        pub fn new_24b(width: u16) -> Self { Self(String::with_capacity(width as usize * 39)) }
        pub fn new_8b(width: u16) -> Self { Self(String::with_capacity(width as usize * 23)) }
        pub fn fg_24b<'a, P>(
                &mut self,
                fg: &'a P,
                color_attributes: &ColorAttributes,
        ) -> fmt::Result
        where
                P: Pixel<Subpixel = u8> + AnsiPixel,
        {
                fg.fg_24b(&mut self.0, color_attributes)
        }
        pub fn bg_24b<'a, P>(
                &mut self,
                bg: &'a P,
                color_attributes: &ColorAttributes,
        ) -> fmt::Result
        where
                P: Pixel<Subpixel = u8> + AnsiPixel,
        {
                bg.bg_24b(&mut self.0, color_attributes)
        }
        pub fn fg_8b<'a, P>(&mut self, fg: &'a P, color_attributes: &ColorAttributes) -> fmt::Result
        where
                P: Pixel<Subpixel = u8> + AnsiPixel,
        {
                fg.fg_8b(&mut self.0, color_attributes)
        }
        pub fn bg_8b<'a, P>(&mut self, bg: &'a P, color_attributes: &ColorAttributes) -> fmt::Result
        where
                P: Pixel<Subpixel = u8> + AnsiPixel,
        {
                bg.bg_8b(&mut self.0, color_attributes)
        }
}

#[derive(Debug, Clone)]
pub struct AnsiImageBuffer {
        pub buf: Vec<AnsiLineBuffer>,
        pub size: (u16, u16),
}
impl AnsiImageBuffer {
        /// Create a new AnsiImageBuffer from a given image and color type and attributes.
        pub fn from(
                img: DynamicImage,
                color_type: &ColorType,
                color_attrs: &ColorAttributes,
        ) -> Self {
                let (width, height) = img.dimensions();
                let size = (width as u16, ((height / 2) + (height % 2)) as u16);
                let mut buf = Vec::with_capacity(size.1 as usize);
                if color_type.is_24bit() {
                        buf.resize_with(size.1 as usize, || AnsiLineBuffer::new_24b(size.0));
                } else {
                        buf.resize_with(size.1 as usize, || AnsiLineBuffer::new_8b(size.0));
                }
                let mut ret = Self { buf, size };
                ret.replace_image(img, color_type, color_attrs);
                ret
        }
        /// Clear each row and reserve more space for updated size.
        fn clear_and_reserve(&mut self, color_type: &ColorType) {
                let n_rows = self.size.1 as usize;
                let n_rows_uninit = n_rows.saturating_sub(self.buf.len());
                // reserve more rows only as needed
                self.buf.reserve(n_rows_uninit);
                // clear rows and reserve space in each row as needed
                let buf_iter = self.buf.iter_mut().take(n_rows);
                if color_type.is_24bit() {
                        buf_iter.for_each(|s| {
                                s.0.clear();
                                s.reserve_24b(self.size.0);
                        });
                        // fill allocated but uninitialized rows
                        self.buf.extend(std::iter::repeat_with(|| {
                                AnsiLineBuffer::new_24b(self.size.0)
                        })
                        .take(n_rows_uninit));
                } else {
                        buf_iter.for_each(|s| {
                                s.0.clear();
                                s.reserve_8b(self.size.0);
                        });
                        // fill allocated but uninitialized rows
                        self.buf.extend(std::iter::repeat_with(|| {
                                AnsiLineBuffer::new_8b(self.size.0)
                        })
                        .take(n_rows_uninit));
                }
        }
        /// Write rows of an image as 24-bit (RGB) ANSI pixels, 2 rows of image pixels per row of ansi
        pub fn replace_rows_24b<P>(
                &mut self,
                img: image::ImageBuffer<P, Vec<P::Subpixel>>,
                color_type: &ColorType,
                color_attrs: &ColorAttributes,
        ) where
                P: Pixel<Subpixel = u8> + AnsiPixel,
        {
                let n_rows = self.size.1 as usize;
                self.clear_and_reserve(color_type);
                // testing parallelization
                #[cfg(feature = "rayon-converter")]
                self.buf.par_iter_mut()
                        .take(n_rows)
                        .zip(img.into_vec()
                                .par_chunks(P::CHANNEL_COUNT as usize * self.size.0 as usize * 2)
                                .map(|v| {
                                        v.chunks_exact(
                                                P::CHANNEL_COUNT as usize * self.size.0 as usize,
                                        )
                                        .map(|v| v.chunks_exact(P::CHANNEL_COUNT as usize))
                                }))
                        .for_each(|(row_buf, mut pxs)| match (pxs.next(), pxs.next()) {
                                (Some(fgs), Some(bgs)) => fgs.zip(bgs).for_each(|(fg, bg)| {
                                        let _ = row_buf.fg_24b(P::from_slice(fg), color_attrs);
                                        let _ = row_buf.bg_24b(P::from_slice(bg), color_attrs);
                                        row_buf.0.push_str(crate::UPPER_HALF_BLOCK);
                                }),
                                (Some(fgs), None) => fgs.for_each(|fg| {
                                        let _ = row_buf.fg_24b(P::from_slice(fg), color_attrs);
                                        row_buf.0.push_str(crate::UPPER_HALF_BLOCK);
                                }),
                                _ => unreachable!("Rows in image does not match height"),
                        });
                #[cfg(not(feature = "rayon-converter"))]
                let mut rows = img.rows();
                #[cfg(not(feature = "rayon-converter"))]
                ::std::iter::repeat_with(move || (rows.next(), rows.next()))
                        .take(n_rows)
                        .zip(self.buf.iter_mut())
                        .for_each(|(pxs, row_buf)| match pxs {
                                (Some(fgs), Some(bgs)) => fgs.zip(bgs).for_each(|(fg, bg)| {
                                        let _ = row_buf.fg_24b(fg, color_attrs);
                                        let _ = row_buf.bg_24b(bg, color_attrs);
                                        row_buf.0.push_str(crate::UPPER_HALF_BLOCK);
                                }),
                                (Some(fgs), None) => fgs.for_each(|fg| {
                                        let _ = row_buf.fg_24b(fg, color_attrs);
                                        row_buf.0.push_str(crate::UPPER_HALF_BLOCK);
                                }),
                                _ => unreachable!("Rows in image does not match height"),
                        });
        }
        /// Write rows of an image as 8-bit (256) ANSI pixels, 2 rows of image pixels per row of ansi
        pub fn replace_rows_8b<P>(
                &mut self,
                img: image::ImageBuffer<P, Vec<P::Subpixel>>,
                color_type: &ColorType,
                color_attrs: &ColorAttributes,
        ) where
                P: Pixel<Subpixel = u8> + AnsiPixel,
        {
                let n_rows = self.size.1 as usize;
                self.clear_and_reserve(color_type);
                // testing parallelization
                #[cfg(feature = "rayon-converter")]
                self.buf.par_iter_mut()
                        .take(n_rows)
                        .zip(img.into_vec()
                                .par_chunks(P::CHANNEL_COUNT as usize * self.size.0 as usize * 2)
                                .map(|v| {
                                        v.chunks_exact(
                                                P::CHANNEL_COUNT as usize * self.size.0 as usize,
                                        )
                                        .map(|v| v.chunks_exact(P::CHANNEL_COUNT as usize))
                                }))
                        .for_each(|(row_buf, mut pxs)| match (pxs.next(), pxs.next()) {
                                (Some(fgs), Some(bgs)) => fgs.zip(bgs).for_each(|(fg, bg)| {
                                        let _ = row_buf.fg_24b(P::from_slice(fg), color_attrs);
                                        let _ = row_buf.bg_24b(P::from_slice(bg), color_attrs);
                                        row_buf.0.push_str(crate::UPPER_HALF_BLOCK);
                                }),
                                (Some(fgs), None) => fgs.for_each(|fg| {
                                        let _ = row_buf.fg_24b(P::from_slice(fg), color_attrs);
                                        row_buf.0.push_str(crate::UPPER_HALF_BLOCK);
                                }),
                                // Do nothing because we don't want to handle thread panics
                                _ => (),
                        });
                #[cfg(not(feature = "rayon-converter"))]
                let mut rows = img.rows();
                #[cfg(not(feature = "rayon-converter"))]
                ::std::iter::repeat_with(move || (rows.next(), rows.next()))
                        .take(n_rows)
                        .zip(self.buf.iter_mut())
                        .for_each(|(pxs, row_buf)| match pxs {
                                (Some(fgs), Some(bgs)) => fgs.zip(bgs).for_each(|(fg, bg)| {
                                        let _ = row_buf.fg_8b(fg, color_attrs);
                                        let _ = row_buf.bg_8b(bg, color_attrs);
                                        row_buf.0.push_str(crate::UPPER_HALF_BLOCK);
                                }),
                                (Some(fgs), None) => fgs.for_each(|fg| {
                                        let _ = row_buf.fg_8b(fg, color_attrs);
                                        row_buf.0.push_str(crate::UPPER_HALF_BLOCK);
                                }),
                                _ => unreachable!("Rows in image does not match height"),
                        })
        }
        /// Replace image in buffer with new image, does NOT resize image.
        pub fn replace_image(
                &mut self,
                img: DynamicImage,
                color_type: &ColorType,
                color_attrs: &ColorAttributes,
        ) {
                let (width, height) = img.dimensions();
                self.size = (width as u16, ((height / 2) + (height % 2)) as u16);
                if img.color().has_color() && color_type.is_color() {
                        let img = img.into_rgb8();
                        if color_type.is_24bit() {
                                self.replace_rows_24b(img, color_type, color_attrs);
                        } else {
                                self.replace_rows_8b(img, color_type, color_attrs);
                        }
                } else {
                        let img = img.into_luma8();
                        if color_type.is_24bit() {
                                self.replace_rows_24b(img, color_type, color_attrs);
                        } else {
                                self.replace_rows_8b(img, color_type, color_attrs);
                        }
                };
        }
}

/// Add terminal ANSI writes to a impl Write
pub trait TerminalImpl
where
        Self: io::Write + Sized,
{
        fn clear_buffer(&mut self) -> io::Result<()> {
                self.write_all(term::CLEAR_BUFFER.as_bytes())
        }
        fn clear_screen(&mut self) -> io::Result<()> {
                self.write_all(term::CLEAR_SCREEN.as_bytes())
        }
        fn clear_line(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_LINE.as_bytes()) }
        fn clear_line_to_end(&mut self) -> io::Result<()> {
                self.write_all(term::CLEAR_LINE_TO_END.as_bytes())
        }
        fn clear_line_to_start(&mut self) -> io::Result<()> {
                self.write_all(term::CLEAR_LINE_TO_START.as_bytes())
        }
        fn clear_screen_to_end(&mut self) -> io::Result<()> {
                self.write_all(term::CLEAR_SCREEN_TO_END.as_bytes())
        }
        fn clear_screen_to_start(&mut self) -> io::Result<()> {
                self.write_all(term::CLEAR_SCREEN_TO_START.as_bytes())
        }
        /// !windows
        fn reset(&mut self) -> io::Result<()> { self.write_all(term::RESET.as_bytes()) }
        fn soft_reset(&mut self) -> io::Result<()> { self.write_all(term::SOFT_RESET.as_bytes()) }
        fn enter_alt_screen(&mut self) -> io::Result<()> {
                self.write_all(term::ENTER_ALT_SCREEN.as_bytes())
        }
        fn exit_alt_screen(&mut self) -> io::Result<()> {
                self.write_all(term::EXIT_ALT_SCREEN.as_bytes())
        }
        fn enable_line_wrap(&mut self) -> io::Result<()> {
                self.write_all(term::ENABLE_LINE_WRAP.as_bytes())
        }
        fn disable_line_wrap(&mut self) -> io::Result<()> {
                self.write_all(term::DISABLE_LINE_WRAP.as_bytes())
        }
        #[cfg(any(windows, unix))]
        #[inline]
        fn enable_raw_mode(&mut self) -> io::Result<()> { ::crossterm::terminal::enable_raw_mode() }
        #[cfg(target_family = "wasm")]
        fn enable_raw_mode(&mut self) -> io::Result<()> {
                self.write_all(term::ENABLE_RAW_MODE.as_bytes())
        }
        #[cfg(any(windows, unix))]
        #[inline]
        fn disable_raw_mode(&mut self) -> io::Result<()> {
                ::crossterm::terminal::disable_raw_mode()
        }
        #[cfg(target_family = "wasm")]
        fn disable_raw_mode(&mut self) -> io::Result<()> {
                self.write_all(term::DISABLE_RAW_MODE.as_bytes())
        }

        /// Set the window title using ansi escape codes
        fn set_title<T: ::std::fmt::Display>(&mut self, title: &T) -> io::Result<()> {
                write!(self, osc!("0;", st!("{}")), title)
        }
        /// Resize the window using ansi escape codes
        fn resize(&mut self, width: u16, height: u16) -> io::Result<()> {
                write!(self, csi!("8;{};{}t"), height, width)
        }
        /// Attempt to read the terminal size in characters
        #[cfg(any(windows, unix))]
        #[inline]
        fn size(&mut self, _: bool) -> io::Result<(u16, u16)> { ::crossterm::terminal::size() }
        /// Attempt to read the terminal size in characters using only ANSI escape sequences
        ///   
        /// It is not guaranteed to work, although more universal than a direct x-term style ANSI window size request "\x1B[18t".  
        /// Works best in raw alternate screen mode.
        /// relies on the user to press enter because we cannot read stdout.
        /// WARNING: this is a blocking call
        #[cfg(target_family = "wasm")]
        fn size(&mut self, quiet: bool) -> io::Result<(u16, u16)> {
                // if terms who don't support cursor report at least export COLUMNS and LINES, then we can use that, even if it's not accurate
                if let Ok(s) = std::env::var("COLUMNS")
                        .and_then(|cols| std::env::var("LINES").map(|lines| (cols, lines)))
                {
                        if let (Ok(cols), Ok(lines)) = (s.0.parse(), s.1.parse()) {
                                return Ok((cols, lines));
                        }
                }
                // otherwise, we can try to get the cursor position, but this is not guaranteed to work and user might have to press enter
                if !quiet {
                        eprintln!("Requesting terminal size report, please press enter when a report appears (e.g. \"^[[40;132R\")");
                        eprintln!("If no report appears, then you may need to set --width and/or --height with --inline.");
                }
                self.cursor_save()?;
                self.cursor_to(Coord::MAX.x, Coord::MAX.y)?;
                self.write_all(
                        [term::REPORT_CURSOR_POSITION, term::RESTORE_CURSOR]
                                .concat()
                                .as_bytes(),
                )?;
                self.flush()?;
                let mut buf = [0; 1];
                let mut s = Vec::<u8>::with_capacity(10);
                loop {
                        stdin().read_exact(&mut buf)?;
                        match buf[0] {
                                b'0'..=b'9' | b';' => s.push(buf[0]),
                                b'\0' | b'\n' | b'\r' | b'R' => break,
                                _ => continue,
                        }
                }
                if let Ok(s) = String::from_utf8(s) {
                        if let Ok(coord) = Coord::try_from_report(&s) {
                                return Ok((coord.x + 1, coord.y + 1));
                        }
                }
                if !quiet {
                        eprintln!(
                                "Failed to parse terminal size report, defaulting to {}x{}",
                                crate::DEFAULT_COLS,
                                crate::DEFAULT_ROWS
                        );
                }
                Ok((crate::DEFAULT_COLS, crate::DEFAULT_ROWS))
        }
        fn cursor_hide(&mut self) -> io::Result<()> { self.write_all(term::HIDE_CURSOR.as_bytes()) }
        fn cursor_show(&mut self) -> io::Result<()> { self.write_all(term::SHOW_CURSOR.as_bytes()) }
        fn cursor_save(&mut self) -> io::Result<()> { self.write_all(term::SAVE_CURSOR.as_bytes()) }
        fn cursor_restore(&mut self) -> io::Result<()> {
                self.write_all(term::RESTORE_CURSOR.as_bytes())
        }
        fn cursor_report_position(&mut self) -> io::Result<()> {
                self.write_all(term::REPORT_CURSOR_POSITION.as_bytes())
        }
        fn cursor_next_line(&mut self) -> io::Result<()> {
                self.write_all(cursor::NEXT_LINE.as_bytes())
        }
        fn cursor_prev_line(&mut self) -> io::Result<()> {
                self.write_all(cursor::PREV_LINE.as_bytes())
        }
        fn cursor_home(&mut self) -> io::Result<()> { self.write_all(cursor::HOME.as_bytes()) }
        fn cursor_to(&mut self, x: u16, y: u16) -> io::Result<()> {
                write!(self, csi!("{};{}H"), y + 1, x + 1)
        }
        fn cursor_to_col(&mut self, x: u16) -> io::Result<()> { write!(self, csi!("{}G"), x + 1) }
        fn cursor_up(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}A"), n) }
        fn cursor_down(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}B"), n) }
        fn cursor_foward(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}C"), n) }
        fn cursor_backward(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}D"), n) }
        fn cursor_next_lines(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}E"), n) }
        fn cursor_prev_lines(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}F"), n) }
        fn attr_reset(&mut self) -> io::Result<()> { self.write_all(attr::RESET.as_bytes()) }
}
impl<'a> TerminalImpl for io::StdoutLock<'a> {}
impl TerminalImpl for io::Stdout {}

pub mod term {
        pub const CLEAR_BUFFER: &str = csi!("3J");
        pub const CLEAR_SCREEN: &str = csi!("2J");
        pub const CLEAR_SCREEN_TO_END: &str = csi!("0J");
        pub const CLEAR_SCREEN_TO_START: &str = csi!("1J");
        pub const CLEAR_LINE: &str = csi!("2K");
        pub const CLEAR_LINE_TO_END: &str = csi!("0K");
        pub const CLEAR_LINE_TO_START: &str = csi!("1K");
        pub const RESET: &str = esc!("c");
        pub const SOFT_RESET: &str = esc!("!p");
        pub const RESET_ATTRIBUTES: &str = csi!("0m");

        pub const ENTER_ALT_SCREEN: &str = csi!("?1049h");
        pub const EXIT_ALT_SCREEN: &str = csi!("?1049l");
        pub const ENABLE_LINE_WRAP: &str = csi!("?7h");
        pub const DISABLE_LINE_WRAP: &str = csi!("?7l");
        /// use crossterm instead when possible
        pub const ENABLE_RAW_MODE: &str = csi!("?1h");
        /// use crossterm instead when possible
        pub const DISABLE_RAW_MODE: &str = csi!("?1l");

        pub const HIDE_CURSOR: &str = csi!("?25l");
        pub const SHOW_CURSOR: &str = csi!("?25h");
        pub const SAVE_CURSOR: &str = esc!("7");
        pub const RESTORE_CURSOR: &str = esc!("8");
        pub const REPORT_CURSOR_POSITION: &str = csi!("6n");

        // pub const START_SIXEL: &str = dcs!("Pq");
}

pub mod cursor {
        pub const HOME: &str = csi!("H");
        pub const NEXT_LINE: &str = csi!("1E");
        pub const PREV_LINE: &str = csi!("1F");
}

// xterm reports
// avoid as much as possible until we get non-blocking stdin
// /// -> `CSI  8 ;  height ;  width t`.
// const REPORT_WINDOW_CHAR_SIZE: &str = csi!("18t");
// /// -> `CSI  9 ;  height ;  width t`.
// const REPORT_SCREEN_CHAR_SIZE: &str = csi!("19t");
// /// -> `OSC  L  label ST`
// const REPORT_WINDOW_ICON_LABEL: &str = csi!("20t");
// /// -> `OSC  l  label ST`
// const REPORT_WINDOW_TITLE: &str = csi!("21t");
