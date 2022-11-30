//! trying my best to make an pure ANSI module for all platforms...
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
//!  - ESC = escape = "\x1B"
//!  - ST = string terminator = "\x1B\\"
//!  - CSI = control sequence introducer = "\x1B["
//!  - OSC = operating system command = "\x1B]"
//!  - DCS = device control string = "\x1BP"
//!  - APM = application program mode = "\x1B_"
//!  - SGR = select graphic rendition = "\x1B[" + _ + "m"

// Maybe make a PR for crossterm to add pure ansi backup when we aren't compiling for UNIX/Windows?

#![allow(dead_code)]

#[cfg(target_family = "wasm")]
use std::io::{stdin, Read};

use color::{AnsiColor, RawPixel};

use self::color::PixelWriter;
use super::{ColorAttributes, ColorType, *};
use crate::Result;

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

#[derive(Debug, Clone, Copy)]
struct Coord {
    x: u16,
    y: u16,
}
impl Coord {
    /// Safe maximum of Coord assuming 16-bit signed integers is used by terminal for cursor position. (Max value is 32767 for c_short)
    const MAX: Coord = Coord { x: 0x7FFF, y: 0x7FFF };
    // fn new(x: u16, y: u16) -> Self { Self { x, y } }
    // fn as_report(&self) -> String { format!("{};{}", self.y, self.x) }
    /// e.g. "1;2" -> Coord { x: 2, y: 1 }
    fn try_from_report(s: &str) -> Result<Self> {
        let mut iter = s.split(';');
        if let (Some(y), Some(x)) = (iter.next(), iter.next()) {
            Ok(Self { x: x.parse()?, y: y.parse()? })
        } else {
            Err(anyhow!("Invalid cursor position report: {}", s))
        }
    }
}

pub mod attr {
    pub const RESET: &str = csi!("0m");
    //...
}

pub mod color;

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

/// Add terminal ANSI writes to a impl Write
pub trait TerminalImpl: io::Write + Sized {
    #[inline]
    fn clear(&mut self) -> io::Result<()> { self.clear_buffer().and_then(|_| self.clear_screen()) }
    #[inline]
    fn clear_buffer(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_BUFFER.as_bytes()) }
    #[inline]
    fn clear_screen(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_SCREEN.as_bytes()) }
    #[inline]
    fn clear_line(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_LINE.as_bytes()) }
    #[inline]
    fn clear_line_to_end(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_LINE_TO_END.as_bytes()) }
    #[inline]
    fn clear_line_to_start(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_LINE_TO_START.as_bytes()) }
    #[inline]
    fn clear_screen_to_end(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_SCREEN_TO_END.as_bytes()) }
    #[inline]
    fn clear_screen_to_start(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_SCREEN_TO_START.as_bytes()) }
    #[inline]
    /// does not work on windows
    fn reset(&mut self) -> io::Result<()> { self.write_all(term::RESET.as_bytes()) }
    #[inline]
    fn soft_reset(&mut self) -> io::Result<()> { self.write_all(term::SOFT_RESET.as_bytes()) }
    #[inline]
    fn enter_alt_screen(&mut self) -> io::Result<()> { self.write_all(term::ENTER_ALT_SCREEN.as_bytes()) }
    #[inline]
    fn exit_alt_screen(&mut self) -> io::Result<()> { self.write_all(term::EXIT_ALT_SCREEN.as_bytes()) }
    #[inline]
    fn enable_line_wrap(&mut self) -> io::Result<()> { self.write_all(term::ENABLE_LINE_WRAP.as_bytes()) }
    #[inline]
    fn disable_line_wrap(&mut self) -> io::Result<()> { self.write_all(term::DISABLE_LINE_WRAP.as_bytes()) }
    #[cfg(any(windows, unix))]
    #[inline]
    fn enable_raw_mode(&mut self) -> io::Result<()> { ::crossterm::terminal::enable_raw_mode() }
    #[cfg(target_family = "wasm")]
    fn enable_raw_mode(&mut self) -> io::Result<()> { self.write_all(term::ENABLE_RAW_MODE.as_bytes()) }
    #[cfg(any(windows, unix))]
    #[inline]
    fn disable_raw_mode(&mut self) -> io::Result<()> { ::crossterm::terminal::disable_raw_mode() }
    #[cfg(target_family = "wasm")]
    #[inline]
    fn disable_raw_mode(&mut self) -> io::Result<()> { self.write_all(term::DISABLE_RAW_MODE.as_bytes()) }

    /// Set the window title using ansi escape codes
    fn set_title<T: ::std::fmt::Display>(&mut self, title: &T) -> io::Result<()> {
        write!(self, osc!("0;", st!("{}")), title)
    }
    /// Resize the window using ansi escape codes
    fn resize(&mut self, width: u16, height: u16) -> io::Result<()> { write!(self, csi!("8;{};{}t"), height, width) }
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
        if let Ok(s) = std::env::var("COLUMNS").and_then(|cols| std::env::var("LINES").map(|lines| (cols, lines))) {
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
        self.write_all([term::REPORT_CURSOR_POSITION, term::RESTORE_CURSOR].concat().as_bytes())?;
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
            eprintln!("Failed to parse terminal size report, defaulting to {}x{}", crate::DEFAULT_COLS, crate::DEFAULT_ROWS);
        }
        Ok((crate::DEFAULT_COLS, crate::DEFAULT_ROWS))
    }
    #[inline]
    fn cursor_hide(&mut self) -> io::Result<()> { self.write_all(term::HIDE_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_show(&mut self) -> io::Result<()> { self.write_all(term::SHOW_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_save(&mut self) -> io::Result<()> { self.write_all(term::SAVE_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_restore(&mut self) -> io::Result<()> { self.write_all(term::RESTORE_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_report_position(&mut self) -> io::Result<()> { self.write_all(term::REPORT_CURSOR_POSITION.as_bytes()) }
    #[inline]
    fn cursor_next_line(&mut self) -> io::Result<()> { self.write_all(cursor::NEXT_LINE.as_bytes()) }
    #[inline]
    fn cursor_prev_line(&mut self) -> io::Result<()> { self.write_all(cursor::PREV_LINE.as_bytes()) }
    #[inline]
    fn cursor_home(&mut self) -> io::Result<()> { self.write_all(cursor::HOME.as_bytes()) }
    fn cursor_to(&mut self, x: u16, y: u16) -> io::Result<()> { write!(self, csi!("{};{}H"), y + 1, x + 1) }
    fn cursor_to_col(&mut self, x: u16) -> io::Result<()> { write!(self, csi!("{}G"), x + 1) }
    fn cursor_up(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}A"), n) }
    fn cursor_down(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}B"), n) }
    fn cursor_foward(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}C"), n) }
    fn cursor_backward(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}D"), n) }
    fn cursor_next_lines(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}E"), n) }
    fn cursor_prev_lines(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}F"), n) }
    #[inline]
    fn attr_reset(&mut self) -> io::Result<()> { self.write_all(attr::RESET.as_bytes()) }
}
impl<'a> TerminalImpl for io::StdoutLock<'a> {}
impl TerminalImpl for io::Stdout {}

#[derive(Debug, Clone)]
/// A single row of an ansi image, representing 2 rows of the real image pixels
pub struct AnsiRow(pub Vec<u8>);
impl AnsiRow {
    #[inline(always)]
    pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional) }
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self { Self(Vec::with_capacity(capacity)) }
    #[inline(always)]
    pub fn clear(&mut self) { self.0.clear() }
    #[inline(always)]
    pub fn reserve_color<C: AnsiColor>(&mut self, additional: usize) {
        self.0.reserve(additional * <C::Writer as color::ColorWriter>::RESERVE_SIZE)
    }
    #[inline]
    pub fn with_capacity_color<C: AnsiColor>(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity * <C::Writer as color::ColorWriter>::RESERVE_SIZE))
    }
    #[inline]
    pub fn extend_fgs_bgs<P: RawPixel, C: AnsiColor>(
        &mut self,
        fgs: ArrayView2<u8>,
        bgs: ArrayView2<u8>,
        attrs: &ColorAttributes,
    ) {
        // unsafe { *(fg.as_ptr() as *const P::Repr) } *should* be safe to do as long as we ensure that the array is the correct size
        fgs.outer_iter().zip(bgs.outer_iter()).for_each(|(fg, bg)| {
            PixelWriter::fg::<P, C>(&mut self.0, unsafe { *(fg.as_ptr() as *const P::Repr) }, attrs);
            PixelWriter::bg::<P, C>(&mut self.0, unsafe { *(bg.as_ptr() as *const P::Repr) }, attrs);
            self.0.extend(crate::UPPER_HALF_BLOCK.as_bytes());
        });
    }
    #[inline]
    pub fn extend_fgs<P: RawPixel, C: AnsiColor>(&mut self, fgs: ArrayView2<u8>, attrs: &ColorAttributes) {
        fgs.outer_iter().for_each(|fg| {
            PixelWriter::fg::<P, C>(&mut self.0, unsafe { *(fg.as_ptr() as *const P::Repr) }, attrs);
            self.0.extend(crate::UPPER_HALF_BLOCK.as_bytes());
        });
    }
    #[inline]
    pub fn as_slice(&self) -> &[u8] { &self.0 }
}

/// An ansi image, 2 rows of pixels per row of ansi
/// "ratchet" memory buffer, so that we minimize memory allocations, because we *should* be relatively memory efficient
// NOTE: A 2D Vec<AnsiRow> is necessary to avoid extreme complexity and manually handling our Vec lengths and such, and it saves some memory too
#[derive(Debug, Clone)]
pub struct AnsiImage {
    buf: Vec<AnsiRow>,
    size: (u16, u16),
}

macro_rules! match_color_as_C {
    ($col: expr, $f: block) => {
        match $col {
            ColorType::Color256 => {
                type C = color::Color8;
                $f
            }
            ColorType::Color => {
                type C = color::Color24;
                $f
            }
            ColorType::Gray256 => {
                type C = color::Gray8;
                $f
            }
            ColorType::Gray => {
                type C = color::Gray24;
                $f
            }
        }
    };
}

impl AnsiImage {
    /// Create a new AnsiImageBuffer from a given image and color type and attributes.
    pub fn new<P: Pixel>(img: &ImageBuffer<P, Vec<u8>>, color_type: &ColorType, color_attrs: &ColorAttributes) -> Self {
        crate::timer!("AnsiImage::new");
        let size = Self::_get_size_from(img.dimensions());
        let buf = Vec::with_capacity(size.1 as usize);
        let mut ret = Self { buf, size };
        match_color_as_C!(color_type, {
            let reserve = size.0 as usize * <<C as AnsiColor>::Writer as color::ColorWriter>::RESERVE_SIZE;
            ret.buf.resize_with(size.1 as usize, || AnsiRow::with_capacity(reserve));
            ret._fill::<P, C>(img, color_attrs);
        });
        ret
    }
    /// Replace image in buffer with new image, assumes image is resized to fit.
    pub fn replace_image<P: Pixel>(
        &mut self,
        img: &ImageBuffer<P, Vec<u8>>,
        color_type: &ColorType,
        color_attrs: &ColorAttributes,
    ) {
        crate::timer!("AnsiImage::replace_image");
        self.size = Self::_get_size_from(img.dimensions());
        match_color_as_C!(color_type, {
            self._pour::<C>();
            self._fill::<P, C>(img, color_attrs);
        });
    }
    #[inline]
    pub fn size(&self) -> &(u16, u16) { &self.size }
    #[inline]
    pub fn rows(&self) -> core::slice::Iter<AnsiRow> {
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr(), self.size.1 as usize) }.iter()
    }
    // No reason to expose this.
    fn rows_mut(&mut self) -> core::slice::IterMut<AnsiRow> {
        unsafe { core::slice::from_raw_parts_mut(self.buf.as_mut_ptr(), self.size.1 as usize) }.iter_mut()
    }
    /// Clear each row within size and reserve more space as needed.
    fn _pour<C: AnsiColor>(&mut self) {
        // clear and reserve rows already initialized within size
        let lines = self.size.1 as usize;
        let res = self.size.0 as usize * <C::Writer as color::ColorWriter>::RESERVE_SIZE;
        self.buf.iter_mut().take(lines).for_each(|s| {
            s.clear();
            s.reserve(res);
        });
        // fill uninitialized rows within size
        let uninit = lines.saturating_sub(self.buf.len());
        self.buf.reserve(uninit);
        self.buf.extend(std::iter::repeat_with(|| AnsiRow::with_capacity(res)).take(uninit));
    }
    /// Write rows of an image as ANSI colors and half block characters, 2 rows of image pixels per row of ansi,
    /// assumes buf is already cleared
    #[cfg(feature = "rayon")]
    fn _fill<P: Pixel, C: AnsiColor>(&mut self, img: &image::ImageBuffer<P, Vec<u8>>, attrs: &ColorAttributes) {
        let (w, h) = img.dimensions();
        self.buf
            .par_iter_mut()
            .take(self.size.1 as usize)
            .zip(
                ArrayView3::from_shape([h as usize, w as usize, P::CHANNELS], img)
                    .expect(concat!(module_path!(), "AnsiImage::_fill: invalid shape"))
                    .axis_chunks_iter(Axis(0), 2)
                    .into_par_iter(),
            )
            .for_each(|(row_buf, pxs)| {
                let mut pxs = pxs.outer_iter();
                match (pxs.next(), pxs.next()) {
                    (Some(fgs), Some(bgs)) => row_buf.extend_fgs_bgs::<P, C>(fgs, bgs, attrs),
                    (Some(fgs), None) => row_buf.extend_fgs::<P, C>(fgs, attrs),
                    _ => unreachable!("Rows in image does not match height"),
                }
            });
    }
    #[cfg(not(feature = "rayon"))]
    fn _fill<P: Pixel, C: AnsiColor>(&mut self, img: &image::ImageBuffer<P, Vec<u8>>, attrs: &ColorAttributes) {
        let (w, h) = img.dimensions();
        let rows = ArrayView3::from_shape([h as usize, w as usize, P::CHANNELS], img)
            .expect(concat!(module_path!(), "AnsiImage::_fill: invalid shape"));
        let mut rows = rows.outer_iter();
        ::std::iter::repeat_with(move || (rows.next(), rows.next()))
            .take(self.size.1 as usize)
            .zip(self.rows_mut())
            .for_each(|(pxs, row_buf)| match pxs {
                (Some(fgs), Some(bgs)) => row_buf.extend_fgs_bgs::<P, C>(fgs, bgs, attrs),
                (Some(fgs), None) => row_buf.extend_fgs::<P, C>(fgs, attrs),
                _ => unreachable!("Rows in image does not match height"),
            });
    }
    /// Get ansi image dimensions from real image dimensions
    #[inline]
    fn _get_size_from((width, height): (u32, u32)) -> (u16, u16) { (width as u16, ((height / 2) + (height % 2)) as u16) }
}

impl<'a> IntoIterator for &'a AnsiImage {
    type Item = &'a AnsiRow;
    type IntoIter = core::slice::Iter<'a, AnsiRow>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter { self.rows() }
}
