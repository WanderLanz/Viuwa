//! The main application as a struct so we don't have to pass around all the arguments
use crate::{err_msg, Args, BoxResult};

use std::io::{self, stdout, StdoutLock, Write};
#[cfg(target_family = "wasm")]
use std::io::{stdin, Read};

use image::DynamicImage;
use image::{ImageBuffer, Luma, Rgb};

pub mod ansi;
pub mod resizer;

use ansi::{AnsiImage, TerminalImpl};
use resizer::{FilterType, Resizer};

use ndarray::prelude::*;
#[cfg(feature = "rayon")]
use rayon::prelude::*;

/// Wrapper around possibly user-controlled color attributes
#[derive(Debug, Clone, Copy)]
pub struct ColorAttributes {
    /// luma correct as a color distance threshold
    pub luma_correct: u32,
}

impl ColorAttributes {
    /// luma correct is 0..=100, 100 is the highest luma correct
    // distance threshold = (MAX_COLOR_DISTANCE / 100) * ((100 - luma_correct)^2 / 100)
    pub fn new(luma_correct: u32) -> Self {
        Self {
            luma_correct: (((100 - luma_correct).pow(2) / 100) as f32 * ansi::color::MAP_0_100_DIST) as u32,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ColorType {
    Color,
    Color256,
    Gray,
    Gray256,
}

impl ColorType {
    #[inline]
    pub fn cycle(&self) -> ColorType {
        match self {
            ColorType::Color => ColorType::Color256,
            ColorType::Color256 => ColorType::Gray,
            ColorType::Gray => ColorType::Gray256,
            ColorType::Gray256 => ColorType::Color,
        }
    }
    #[inline]
    pub fn is_color(&self) -> bool {
        match self {
            ColorType::Color | ColorType::Color256 => true,
            ColorType::Gray | ColorType::Gray256 => false,
        }
    }
    #[inline]
    pub fn is_24bit(&self) -> bool {
        match self {
            ColorType::Color | ColorType::Gray => true,
            ColorType::Color256 | ColorType::Gray256 => false,
        }
    }
}
// /// For when and if we decide to add more TUI features and want to abstract away the cli args
// pub struct DynamicVars {
//         pub color_type: ColorType,
//         pub color_attrs: ColorAttributes,
//         pub filter: FilterType,
// }

pub struct Viuwa<'a> {
    /// The image to display
    pub resizer: Resizer,
    /// The ANSI buffer
    pub ansi: AnsiImage,
    /// The terminal size in columns and rows
    pub size: (u16, u16),
    /// Lock to stdout
    pub lock: StdoutLock<'a>,
    pub args: Args,
}

impl<'a> Viuwa<'a> {
    /// Create a new viuwa instance
    pub fn new(orig: DynamicImage, args: Args) -> BoxResult<Self> {
        let mut lock = stdout().lock();
        let size = lock.size(args.quiet)?;
        let resizer = if orig.color().has_color() {
            Resizer::from_rgb(orig.into_rgb8(), &args.filter, (size.0 as u32, size.1 as u32 * 2))
        } else {
            Resizer::from_luma(orig.into_luma8(), &args.filter, (size.0 as u32, size.1 as u32 * 2))
        };
        let ansi = AnsiImage::from(resizer.resized(), &args.color, &ColorAttributes::new(args.luma_correct));
        Ok(Self {
            resizer,
            ansi,
            size,
            lock,
            args,
        })
    }
    // seperate spawns because of resize event
    /// Start viuwa app
    #[cfg(any(unix, windows))]
    pub fn spawn(mut self) -> BoxResult<()> {
        use crossterm::event::{Event, KeyCode, KeyEventKind};
        self.lock.enable_raw_mode()?;
        self.lock.write_all(
            [
                ansi::term::ENTER_ALT_SCREEN,
                ansi::term::HIDE_CURSOR,
                ansi::term::DISABLE_LINE_WRAP,
                ansi::term::CLEAR_BUFFER,
            ]
            .concat()
            .as_bytes(),
        )?;
        self.lock.flush()?;
        self._draw()?;
        loop {
            match crossterm::event::read()? {
                Event::Key(e) if e.kind == KeyEventKind::Press => match e.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => {
                        self._draw()?;
                    }
                    KeyCode::Char('h') => {
                        self._help()?;
                        self._draw()?;
                    }
                    KeyCode::Char('f') => {
                        self._cycle_filter();
                        self._draw()?;
                    }
                    KeyCode::Char('c') => {
                        self._cycle_color();
                        self._draw()?;
                    }
                    KeyCode::Char('1') => {
                        self.args.color = ColorType::Color;
                        self._rebuild_buf();
                        self._draw()?;
                    }
                    KeyCode::Char('2') => {
                        self.args.color = ColorType::Color256;
                        self._rebuild_buf();
                        self._draw()?;
                    }
                    KeyCode::Char('3') => {
                        self.args.color = ColorType::Gray;
                        self._rebuild_buf();
                        self._draw()?;
                    }
                    KeyCode::Char('4') => {
                        self.args.color = ColorType::Gray256;
                        self._rebuild_buf();
                        self._draw()?;
                    }
                    KeyCode::Char('!') => {
                        self.resizer.filter(FilterType::Nearest);
                        self._rebuild_buf();
                        self._draw()?;
                    }
                    KeyCode::Char('@') => {
                        self.resizer.filter(FilterType::Triangle);
                        self._rebuild_buf();
                        self._draw()?;
                    }
                    KeyCode::Char('#') => {
                        self.resizer.filter(FilterType::CatmullRom);
                        self._rebuild_buf();
                        self._draw()?;
                    }
                    KeyCode::Char('$') => {
                        self.resizer.filter(FilterType::Gaussian);
                        self._rebuild_buf();
                        self._draw()?;
                    }
                    KeyCode::Char('%') => {
                        self.resizer.filter(FilterType::Lanczos3);
                        self._rebuild_buf();
                        self._draw()?;
                    }
                    _ => {}
                },
                Event::Resize(w, h) => {
                    self.lock.clear()?;
                    self.lock.flush()?;
                    self._handle_resize(w, h);
                    self._draw()?;
                }
                _ => (),
            }
        }
        self.lock.write_all(
            [
                ansi::term::ENABLE_LINE_WRAP,
                ansi::term::SHOW_CURSOR,
                ansi::term::EXIT_ALT_SCREEN,
            ]
            .concat()
            .as_bytes(),
        )?;
        self.lock.disable_raw_mode()?;
        self.lock.flush()?;
        Ok(())
    }
    /// Start viuwa app
    #[cfg(target_family = "wasm")]
    pub fn spawn(mut self) -> BoxResult<()> {
        self.lock.enable_raw_mode()?;
        self.lock.write_all(
            [
                ansi::term::ENTER_ALT_SCREEN,
                ansi::term::HIDE_CURSOR,
                ansi::term::DISABLE_LINE_WRAP,
            ]
            .concat()
            .as_bytes(),
        )?;
        self.lock.flush()?;
        self._draw()?;
        let mut buf = [0; 1];
        loop {
            stdin().read_exact(&mut buf)?;
            match buf[0] {
                b'q' => break,
                b'r' => {
                    let size = self.lock.size(self.args.quiet)?;
                    self._handle_resize(size.0, size.1);
                    self._draw()?;
                }
                b'h' => {
                    self._help()?;
                    self._draw()?;
                }
                b'f' => {
                    self._cycle_filter();
                    self._draw()?;
                }
                b'c' => {
                    self._cycle_color();
                    self._draw()?;
                }
                b'1' => {
                    self.args.color = ColorType::Color;
                    self._rebuild_buf();
                    self._draw()?;
                }
                b'2' => {
                    self.args.color = ColorType::Color256;
                    self._rebuild_buf();
                    self._draw()?;
                }
                b'3' => {
                    self.args.color = ColorType::Gray;
                    self._rebuild_buf();
                    self._draw()?;
                }
                b'4' => {
                    self.args.color = ColorType::Gray256;
                    self._rebuild_buf();
                    self._draw()?;
                }
                b'!' => {
                    self.resizer.filter(FilterType::Nearest);
                    self._rebuild_buf();
                    self._draw()?;
                }
                b'@' => {
                    self.resizer.filter(FilterType::Triangle);
                    self._rebuild_buf();
                    self._draw()?;
                }
                b'#' => {
                    self.resizer.filter(FilterType::CatmullRom);
                    self._rebuild_buf();
                    self._draw()?;
                }
                b'$' => {
                    self.resizer.filter(FilterType::Gaussian);
                    self._rebuild_buf();
                    self._draw()?;
                }
                b'%' => {
                    self.resizer.filter(FilterType::Lanczos3);
                    self._rebuild_buf();
                    self._draw()?;
                }
                _ => {}
            }
        }
        self.lock.write_all(
            [
                ansi::term::ENABLE_LINE_WRAP,
                ansi::term::SHOW_CURSOR,
                ansi::term::EXIT_ALT_SCREEN,
            ]
            .concat()
            .as_bytes(),
        )?;
        self.lock.disable_raw_mode()?;
        self.lock.flush()?;
        Ok(())
    }
    /// Write the buffer to the terminal, and move the cursor to the bottom left
    fn _draw(&mut self) -> BoxResult<()> {
        self.lock.clear()?;
        let ox = (self.size.0 - self.ansi.size().0) / 2;
        let oy = (self.size.1 - self.ansi.size().1) / 2;
        for (y, row) in self.ansi.rows().enumerate() {
            self.lock.cursor_to(ox, oy + y as u16)?;
            self.lock.write_all(row.as_slice())?;
            self.lock.attr_reset()?;
        }
        self.lock.cursor_to(0, self.size.1)?;
        self.lock.flush()?;
        Ok(())
    }
    /// clear screen, print help, and quit 'q'
    fn _help(&mut self) -> BoxResult<()> {
        self.lock.clear()?;
        self.lock.cursor_home()?;
        self._write_centerx(0, "Viuwa help:")?;
        self._write_centerxy_align_all(
            &[
                "[q]: quit",
                "[r]: redraw",
                "[h]: help",
                "[c]: cycle color",
                "[f]: cycle filter",
                "[1]: set color to Truecolor",
                "[2]: set color to 256",
                "[3]: set color to Gray",
                "[4]: set color to 256Gray",
                "[Shift + 1]: set filter to nearest",
                "[Shift + 2]: set filter to triangle",
                "[Shift + 3]: set filter to catmull rom",
                "[Shift + 4]: set filter to gaussian",
                "[Shift + 5]: set filter to lanczos3",
            ]
            .to_vec(),
        )?;
        self.lock.cursor_to(0, self.size.1)?;
        self.lock.flush()?;
        wait_for_quit()
    }
    /// handle resize event
    fn _handle_resize(&mut self, w: u16, h: u16) {
        let nsz = (w + 1, h + 1);
        if nsz != self.size {
            self.size = nsz;
            self._rebuild_buf();
        }
    }
    /// print a string centered on the x axis
    fn _write_centerx(&mut self, y: u16, s: &str) -> io::Result<()> {
        self.lock.cursor_to((self.size.0 - s.len() as u16) / 2, y)?;
        self.lock.write_all(s.as_bytes())?;
        Ok(())
    }
    /// print strings centered and aligned on the x axis and y axis
    fn _write_centerxy_align_all(&mut self, s: &Vec<&str>) -> BoxResult<()> {
        if let Some(max) = s.iter().map(|x| x.len()).max() {
            let ox = (self.size.0 - max as u16) / 2;
            let oy = (self.size.1 - s.len() as u16) / 2;
            for (i, line) in s.iter().enumerate() {
                self.lock.cursor_to(ox, oy + i as u16)?;
                self.lock.write_all(line.as_bytes())?;
            }
            Ok(())
        } else {
            Err(err_msg!("0 args to _write_centerxy_align_all").into())
        }
    }
    /// Cycle through filter types, and rebuild buffer
    fn _cycle_filter(&mut self) {
        self.resizer.cycle_filter();
        self._rebuild_buf();
    }
    /// Cycle through output formats, and rebuild buffer
    fn _cycle_color(&mut self) {
        self.args.color = self.args.color.cycle();
        self._rebuild_buf();
    }
    /// Rebuild the buffer with the current image, filter, and format
    fn _rebuild_buf(&mut self) {
        self.resizer.resize(self.size.0 as u32, self.size.1 as u32 * 2);
        self.ansi.replace_image(
            self.resizer.resized(),
            &self.args.color,
            &ColorAttributes::new(self.args.luma_correct),
        );
    }
}

#[cfg(target_family = "wasm")]
fn wait_for_quit() -> BoxResult<()> {
    let mut buf = [0; 1];
    let mut stdin = stdin().lock();
    loop {
        stdin.read_exact(&mut buf)?;
        match buf[0] {
            b'q' => break,
            _ => (),
        }
    }
    Ok(())
}
#[cfg(any(windows, unix))]
fn wait_for_quit() -> BoxResult<()> {
    loop {
        match crossterm::event::read()? {
            crossterm::event::Event::Key(crossterm::event::KeyEvent {
                code: crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc,
                ..
            }) => break,
            _ => continue,
        }
    }
    Ok(())
}

/// Print ANSI image to stdout without attempting to use alternate screen buffer or other fancy stuff
pub fn inline(orig: DynamicImage, args: Args) -> BoxResult<()> {
    let size = match (args.width, args.height) {
        (None, None) => stdout().size(args.quiet)?,
        (None, Some(h)) => (crate::MAX_COLS, h),
        (Some(w), None) => (w, crate::MAX_ROWS),
        (Some(w), Some(h)) => (w, h),
    };
    let resizer = if orig.color().has_color() {
        Resizer::from_rgb(orig.into_rgb8(), &args.filter, (size.0 as u32, size.1 as u32 * 2))
    } else {
        Resizer::from_luma(orig.into_luma8(), &args.filter, (size.0 as u32, size.1 as u32 * 2))
    };
    let ansi = AnsiImage::from(resizer.resized(), &args.color, &ColorAttributes::new(args.luma_correct));
    let mut lock = stdout().lock();
    for row in &ansi {
        lock.write_all(row.as_slice())?;
        lock.attr_reset()?;
        lock.write_all(b"\n")?;
    }
    lock.flush()?;
    Ok(())
}
