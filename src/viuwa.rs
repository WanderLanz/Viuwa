//! The main application as a struct so we don't have to pass around all the arguments
use std::io::{self, stdout, StdoutLock, Write};
#[cfg(target_family = "wasm")]
use std::io::{stdin, Read};

use image::{DynamicImage, ImageBuffer};

use crate::{Args, Result};

pub mod ansi;
pub mod resizer;

use ansi::{AnsiImage, TerminalImpl};
use anyhow::anyhow;
use ndarray::prelude::*;
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use resizer::{FilterType, Resizer};

pub trait Pixel: ::image::Pixel<Subpixel = u8> + ansi::color::RawPixel {}
impl<T: ::image::Pixel<Subpixel = u8> + ansi::color::RawPixel> Pixel for T {}

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
        Self { luma_correct: (((100 - luma_correct).pow(2) / 100) as f32 * ansi::color::MAP_0_100_DIST) as u32 }
    }
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
pub enum ColorType {
    #[default]
    #[clap(name = "truecolor")]
    Color,
    #[clap(name = "256")]
    Color256,
    #[clap(name = "gray")]
    Gray,
    #[clap(name = "256gray")]
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

pub struct Viuwa<'a, P: Pixel> {
    /// The image to display
    pub resizer: Resizer<P>,
    /// The ANSI buffer
    pub ansi: AnsiImage,
    /// The terminal size in columns and rows
    pub size: (u16, u16),
    /// Lock to stdout
    pub lock: StdoutLock<'a>,
    pub args: Args,
}

impl<'a, P: Pixel> Viuwa<'a, P> {
    /// Create a new viuwa instance
    pub fn new(orig: ImageBuffer<P, Vec<u8>>, args: Args) -> Result<Self> {
        crate::timer!("Viuwa::new");
        let mut lock = stdout().lock();
        let size = lock.size(args.quiet)?;
        let resizer = Resizer::new(orig, &args.filter, (size.0 as u32, size.1 as u32 * 2));
        let ansi = AnsiImage::new(resizer.resized(), &args.color, &ColorAttributes::new(args.luma_correct));
        Ok(Self { resizer, ansi, size, lock, args })
    }
    // seperate spawns because of resize event
    /// Start viuwa app
    #[cfg(any(unix, windows))]
    pub fn spawn(mut self) -> Result<()> {
        crate::timer!("Viuwa::spawn");
        self.lock.enable_raw_mode()?;
        self.lock.enter_alt_screen()?;
        self.lock.cursor_hide()?;
        self.lock.disable_line_wrap()?;
        self.lock.flush()?;
        self._draw()?;
        self._spawn_loop()?;
        self.lock.enable_line_wrap()?;
        self.lock.cursor_show()?;
        self.lock.exit_alt_screen()?;
        self.lock.disable_raw_mode()?;
        self.lock.flush()?;
        Ok(())
    }
    #[cfg(any(unix, windows))]
    fn _spawn_loop(&mut self) -> Result<()> {
        use crossterm::event::{Event, KeyCode, KeyEventKind};
        loop {
            match crossterm::event::read()? {
                Event::Key(e) if e.kind == KeyEventKind::Press => match e.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char(c) => {
                        self._handle_keypress(c as u8)?;
                    }
                    _ => (),
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
        Ok(())
    }
    #[cfg(target_family = "wasm")]
    fn _spawn_loop(&mut self) -> Result<()> {
        let mut buf = [0; 1];
        loop {
            stdin().read_exact(&mut buf)?;
            match buf[0] {
                b'q' => break,
                _ => {
                    self._handle_keypress(buf[0])?;
                }
            }
        }
        Ok(())
    }
    fn _handle_keypress(&mut self, b: u8) -> Result<()> {
        match b {
            b'r' => {}
            b'h' => {
                self._help()?;
            }
            b'f' => {
                self._cycle_filter();
            }
            b'c' => {
                self._cycle_color();
            }
            b'1' => {
                self.args.color = ColorType::Color;
                self._rebuild_buf();
            }
            b'2' => {
                self.args.color = ColorType::Color256;
                self._rebuild_buf();
            }
            b'3' => {
                self.args.color = ColorType::Gray;
                self._rebuild_buf();
            }
            b'4' => {
                self.args.color = ColorType::Gray256;
                self._rebuild_buf();
            }
            b'!' => {
                self.resizer.filter(FilterType::from(0));
                self._rebuild_buf();
            }
            b'@' => {
                self.resizer.filter(FilterType::from(1));
                self._rebuild_buf();
            }
            b'#' => {
                self.resizer.filter(FilterType::from(2));
                self._rebuild_buf();
            }
            b'$' => {
                self.resizer.filter(FilterType::from(3));
                self._rebuild_buf();
            }
            b'%' => {
                self.resizer.filter(FilterType::from(4));
                self._rebuild_buf();
            }
            b'^' => {
                self.resizer.filter(FilterType::from(5));
                self._rebuild_buf();
            }
            b'&' => {
                self.resizer.filter(FilterType::from(6));
                self._rebuild_buf();
            }
            _ => {
                return Ok(());
            }
        };
        self._draw()
    }
    /// Write the buffer to the terminal, and move the cursor to the bottom left
    fn _draw(&mut self) -> Result<()> {
        crate::timer!("Viuwa::_draw");
        #[cfg(not(feature = "profiler"))]
        {
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
        }
        Ok(())
    }
    /// clear screen, print help, and quit 'q'
    fn _help(&mut self) -> Result<()> {
        self.lock.clear()?;
        self.lock.cursor_home()?;
        self._write_centerx(0, "Viuwa help:")?;
        #[cfg(feature = "fir")]
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
                "[Shift + 2]: set filter to box",
                "[Shift + 3]: set filter to triangle",
                "[Shift + 4]: set filter to hamming",
                "[Shift + 5]: set filter to catmull-rom",
                "[Shift + 6]: set filter to mitchell-netravali",
                "[Shift + 7]: set filter to lanczos3",
            ]
            .to_vec(),
        )?;
        #[cfg(not(feature = "fir"))]
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
                "[Shift + 2]: set filter to box",
                "[Shift + 3]: set filter to triangle",
                "[Shift + 4]: set filter to catmull-rom",
                "[Shift + 5]: set filter to mitchell-netravali",
                "[Shift + 6]: set filter to gaussian",
                "[Shift + 7]: set filter to lanczos3",
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
    fn _write_centerxy_align_all(&mut self, s: &Vec<&str>) -> Result<()> {
        if let Some(max) = s.iter().map(|x| x.len()).max() {
            let ox = (self.size.0 - max as u16) / 2;
            let oy = (self.size.1 - s.len() as u16) / 2;
            for (i, line) in s.iter().enumerate() {
                self.lock.cursor_to(ox, oy + i as u16)?;
                self.lock.write_all(line.as_bytes())?;
            }
            Ok(())
        } else {
            Err(anyhow!("0 args to _write_centerxy_align_all"))
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
        crate::timer!("rebuild_buf");
        self.resizer.resize(self.size.0 as u32, self.size.1 as u32 * 2);
        self.ansi.replace_image(self.resizer.resized(), &self.args.color, &ColorAttributes::new(self.args.luma_correct));
    }
}

#[cfg(target_family = "wasm")]
fn wait_for_quit() -> Result<()> {
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
fn wait_for_quit() -> Result<()> {
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
pub fn inlined(orig: DynamicImage, args: Args) -> Result<()> {
    crate::timer!("viuwa::inlined");
    let size = match (args.width, args.height) {
        (None, None) => stdout().size(args.quiet)?,
        (None, Some(h)) => (crate::MAX_COLS, h),
        (Some(w), None) => (w, crate::MAX_ROWS),
        (Some(w), Some(h)) => (w, h),
    };
    if orig.color().has_color() {
        let resizer = Resizer::new(orig.into_rgb8(), &args.filter, (size.0 as u32, size.1 as u32 * 2));
        // resizer.resized().save(format!("nasa-{}x{}.png", size.0 as u32, size.1 as u32 * 2).as_str())?;
        let ansi = AnsiImage::new(resizer.resized(), &args.color, &ColorAttributes::new(args.luma_correct));
        #[cfg(not(feature = "profiler"))]
        {
            let mut lock = stdout().lock();
            for row in &ansi {
                lock.write_all(row.as_slice())?;
                lock.attr_reset()?;
                lock.write_all(b"\n")?;
            }
            lock.flush()?;
        }
    } else {
        let resizer = Resizer::new(orig.into_luma8(), &args.filter, (size.0 as u32, size.1 as u32 * 2));
        let ansi = AnsiImage::new(resizer.resized(), &args.color, &ColorAttributes::new(args.luma_correct));
        #[cfg(not(feature = "profiler"))]
        {
            let mut lock = stdout().lock();
            for row in &ansi {
                lock.write_all(row.as_slice())?;
                lock.attr_reset()?;
                lock.write_all(b"\n")?;
            }
            lock.flush()?;
        }
    }
    Ok(())
}
/// Create a new viuwa instance
pub fn windowed<'a>(orig: DynamicImage, args: Args) -> Result<()> {
    crate::timer!("Viuwa::windowed");
    if orig.color().has_color() {
        Viuwa::new(orig.into_rgb8(), args)?.spawn()
    } else {
        Viuwa::new(orig.into_luma8(), args)?.spawn()
    }
}
