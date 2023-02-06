use std::{
    cell::Cell,
    collections::BTreeMap,
    io::{self, stdout, BufWriter, StdoutLock, Write},
    path::PathBuf,
    str::FromStr,
};

#[cfg(not(target_os = "wasi"))]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use image::{DynamicImage, GenericImageView, ImageBuffer};
use serde::{de, Deserialize};
use viuwa_ansi::{
    execute, fg, image::AnsiRow, AnsiImage, ColorAttributes, ColorDepth, ColorSpace, ColorType, Converter, DynamicAnsiImage,
    Terminal,
};
use viuwa_image::{CompatPixelRepr, CompatScalar, FilterType, Image, ImageOps, ImageView, PixelRepr};

#[macro_use]
mod macros;
mod config;
pub use config::*;
mod commands;
use anyhow::{anyhow, Context, Result};
pub use commands::*;

#[cfg(feature = "trace")]
mod tracing {
    use core::mem::ManuallyDrop;
    pub struct DropFn<F: FnOnce()>(ManuallyDrop<F>);
    impl<F: FnOnce()> DropFn<F> {
        #[inline]
        pub fn new(f: F) -> Self { Self(ManuallyDrop::new(f)) }
    }
    impl<F: FnOnce()> From<F> for DropFn<F> {
        #[inline]
        fn from(f: F) -> Self { Self::new(f) }
    }
    impl<F: FnOnce()> Drop for DropFn<F> {
        #[inline]
        fn drop(&mut self) { (unsafe { ManuallyDrop::take(&mut self.0) })(); }
    }
}
#[cfg(not(target_os = "wasi"))]
use commands::KeyBind;
#[cfg(feature = "trace")]
pub use tracing::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LogLevel {
    /// Info and above
    #[default]
    Info = 0,
    /// Warnings and above
    Warn = 1,
    /// Errors and above
    Error = 2,
    /// No logging
    Silent = 3,
}
impl FromStr for LogLevel {
    type Err = String;
    #[inline]
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            "silent" => Ok(Self::Silent),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}
impl<'de> Deserialize<'de> for LogLevel {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?.parse().map_err(serde::de::Error::custom)
    }
}
impl LogLevel {
    #[inline]
    pub fn enabled(self) -> bool { LOG_LEVEL.with(|cell| cell.get() <= self) }
}
impl From<u8> for LogLevel {
    #[inline]
    fn from(v: u8) -> Self { unsafe { core::mem::transmute(v.min(3)) } }
}

thread_local! {
    pub static LOG_LEVEL: Cell<LogLevel> = Cell::new(LogLevel::Info);
}

pub trait Pixel:
    image::Pixel<Subpixel = <Self as viuwa_image::Pixel>::Scalar>
    + viuwa_ansi::AnsiPixel
    + viuwa_image::CompatPixel
    + viuwa_image::Pixel
where
    Self::Scalar: CompatScalar,
    Self::Repr: CompatPixelRepr,
{
}
impl<
        P: image::Pixel<Subpixel = <Self as viuwa_image::Pixel>::Scalar>
            + viuwa_ansi::AnsiPixel
            + viuwa_image::CompatPixel
            + viuwa_image::Pixel,
    > Pixel for P
where
    P::Scalar: CompatScalar,
    P::Repr: CompatPixelRepr,
{
}

#[inline(always)]
fn div_ceil2(n: usize) -> usize { (n >> 1) + (n & 1) }

pub struct Viuwa<'a, P: Pixel>
where
    P::Scalar: CompatScalar,
    P::Repr: CompatPixelRepr,
{
    pub conf: Config,
    /// The image to display
    pub orig: Image<P>,
    /// The resized image
    pub buf: Image<P>,
    /// The terminal size in columns and rows
    pub sz: (u16, u16),
    /// Lock to stdout
    pub lock: BufWriter<StdoutLock<'a>>,
    /// The current attributes
    pub attrs: ColorAttributes,
}

impl<'a, P: Pixel> Viuwa<'a, P>
where
    P::Scalar: CompatScalar,
    P::Repr: CompatPixelRepr,
{
    /// Create a new viuwa instance
    pub fn new(orig: ImageBuffer<P, Vec<P::Scalar>>, conf: Config) -> Result<Self> {
        trace!("Viuwa::new");
        let attrs = ColorAttributes::new(conf.luma_correct as u32);
        let mut lock = stdout().lock();
        let sz = terminal_size(&mut lock, &conf)?;
        let orig = Image::from(orig);
        let dims = dimensions(sz, &conf, orig.dimensions());
        let buf = {
            #[cfg(feature = "fir")]
            {
                orig.fir_superscale(dims.0, dims.1, &conf.filter, 3)
            }
            #[cfg(not(feature = "fir"))]
            {
                orig.superscale(dims.0, dims.1, &conf.filter, 3.)
            }
        };
        Ok(Self { conf, orig, buf, sz, lock: BufWriter::new(lock), attrs })
    }
    /// Start viuwa app
    pub fn spawn(mut self) {
        trace!("Viuwa::spawn");
        execute!(self.lock, enable_raw_mode(), enter_alt_screen(), cursor_hide(), disable_line_wrap(), flush())
            .expect("Failed to setup Viuwa loop");
        self._draw();
        self._spawn_loop();
        execute!(self.lock, enable_line_wrap(), cursor_show(), exit_alt_screen(), disable_raw_mode(), soft_reset(), flush())
            .expect("Failed to cleanup Viuwa loop");
    }
    fn _spawn_loop(&mut self) {
        #[cfg(not(target_os = "wasi"))]
        {
            loop {
                match crossterm::event::read().expect("Failed to read event") {
                    Event::Key(e) if e.kind == KeyEventKind::Press => {
                        if e.code == KeyCode::Char(':') {
                            if let Some(cmd) = self.parse_command() {
                                match cmd {
                                    Command::Quit => break,
                                    _ => self.command(cmd),
                                }
                            }
                        } else if let Some(cmd) = self.conf.keybinds.get(&KeyBind(e)) {
                            match cmd {
                                Action::Quit => break,
                                _ => self.command(cmd.clone().into()),
                            }
                        }
                    }
                    Event::Resize(w, h) => {
                        if w.saturating_sub(self.sz.0) > 1 || h.saturating_sub(self.sz.1) > 1 {
                            self.sz = (w, h);
                            self.reload();
                        }
                    }
                    _ => (),
                }
            }
        }
        #[cfg(target_os = "wasi")]
        {
            use std::io::BufRead;

            use rustix::{fd::BorrowedFd, io::*};
            let stdin_raw = unsafe { BorrowedFd::borrow_raw(0) };
            let mut stdin = std::io::stdin().lock();
            let mut buf = String::new();
            loop {
                while let Ok(0) = ioctl_fionread(stdin_raw) {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                buf.clear();
                if stdin.read_line(&mut buf).expect("failed to read stdin") == 0 {
                    break;
                }
                let key = buf.trim_end_matches(['\r', '\n']);
                if key == ":" {
                    if let Some(cmd) = self.parse_command() {
                        match cmd {
                            Command::Quit => break,
                            _ => self.command(cmd),
                        }
                    }
                } else if let Some(cmd) = self.conf.keybinds.get(key) {
                    let cmd = cmd.clone();
                    if cmd == Action::Quit {
                        break;
                    } else {
                        self.command(cmd.into());
                    }
                }
            }
        }
    }
    pub fn command(&mut self, cmd: Command) {
        match cmd {
            Command::Quit => (),
            Command::Help => self.help(),
            Command::Refresh => self.refresh(),
            Command::Reload => self.reload(),
            Command::Set(inner) => match inner {
                Setting::Log(level) => self.conf.log = level,
                Setting::Filter(filter) => self.conf.filter = filter,
                Setting::ColorSpace(space) => {
                    if self.conf.color.space() != space {
                        self.conf.color = self.conf.color.cycle_space();
                        self.refresh();
                    }
                }
                Setting::ColorDepth(depth) => {
                    if self.conf.color.depth() != depth {
                        self.conf.color = self.conf.color.cycle_depth();
                        self.refresh();
                    }
                }
                Setting::Color(color) => {
                    if self.conf.color != color {
                        self.conf.color = color;
                        self.refresh();
                    }
                }
                Setting::Width(width) => {
                    if self.conf.width != width {
                        self.conf.width = width;
                        self.reload();
                    }
                }
                Setting::Height(height) => {
                    if self.conf.height != height {
                        self.conf.height = height;
                        self.reload();
                    }
                }
                Setting::LumaCorrect(correct) => {
                    if self.conf.luma_correct != correct {
                        self.conf.luma_correct = correct;
                        self.refresh();
                    }
                }
            },
            Command::Bind(key, command) => {
                let _ = self.conf.keybinds.insert(key, command);
            }
            Command::Unbind(key) => {
                let _ = self.conf.keybinds.remove(&key);
            }
            Command::Cycle(Cyclic::Filter) => {
                self.conf.filter = self.conf.filter.cycle();
                self.reload()
            }
            Command::Cycle(Cyclic::Color) => {
                self.conf.color = self.conf.color.cycle();
                self.refresh()
            }
            Command::Cycle(Cyclic::ColorDepth) => {
                self.conf.color = self.conf.color.cycle_depth();
                self.refresh()
            }
            Command::Cycle(Cyclic::ColorSpace) => {
                self.conf.color = self.conf.color.cycle_space();
                self.refresh()
            }
        };
    }
    fn parse_command(&mut self) -> Option<Command> {
        #[cfg(not(target_os = "wasi"))]
        {
            self.lock.cursor_to(0, self.sz.1 - 1).unwrap();
            self.lock.clear_line().unwrap();
            self.lock.cursor_show().unwrap();
            let mut buf = String::from(":");
            self.lock.write_all(buf.as_bytes()).unwrap();
            self.lock.flush().unwrap();
            let mut idx = 1;
            fn render(lock: &mut impl Terminal, buf: &str, idx: usize) {
                lock.cursor_to_col(0).unwrap();
                lock.clear_line().unwrap();
                lock.write_all(buf.as_bytes()).unwrap();
                lock.cursor_to_col(idx as u16).unwrap();
            }
            loop {
                match crossterm::event::read().unwrap() {
                    Event::Key(KeyEvent { code, kind: KeyEventKind::Press, .. }) => match code {
                        KeyCode::Char(c) => {
                            if idx == buf.len() {
                                buf.push(c);
                                write!(self.lock, "{}", c).unwrap();
                            } else {
                                buf.insert(idx, c);
                                render(&mut self.lock, &buf, idx + 1);
                            }
                            self.lock.flush().unwrap();
                            idx += 1;
                        }
                        KeyCode::Backspace => {
                            if idx > 1 {
                                if idx == buf.len() {
                                    buf.pop();
                                    self.lock.cursor_backward(1).unwrap();
                                    self.lock.clear_line_to_end().unwrap();
                                } else {
                                    buf.remove(idx - 1);
                                    render(&mut self.lock, &buf, idx - 1);
                                }
                                idx -= 1;
                            }
                        }
                        KeyCode::Left => {
                            if idx > 1 {
                                idx -= 1;
                                self.lock.cursor_backward(1).unwrap();
                            }
                        }
                        KeyCode::Right => {
                            if idx < buf.len() {
                                idx += 1;
                                self.lock.cursor_forward(1).unwrap();
                            }
                        }
                        KeyCode::Enter => {
                            self.lock.clear_line().unwrap();
                            self.lock.cursor_hide().unwrap();
                            self.lock.flush().unwrap();
                            break;
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }
            let cmd = buf.trim_start_matches(':');
            self.lock.clear_line().unwrap();
            self.lock.flush().unwrap();
            return match Command::from_str(cmd) {
                Ok(cmd) => Some(cmd),
                Err(e) => {
                    write!(self.lock, "error: {}", e).unwrap();
                    self.lock.flush().unwrap();
                    None
                }
            };
        }
        #[cfg(target_os = "wasi")]
        {
            self.lock.clear_line().unwrap();
            self.lock.cursor_show().unwrap();
            write!(self.lock, ":").unwrap();
            self.lock.flush().unwrap();
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf).unwrap();
            let cmd = buf.trim_end_matches(['\r', '\n']);
            self.lock.clear_line().unwrap();
            self.lock.flush().unwrap();
            return match Command::from_str(cmd) {
                Ok(cmd) => Some(cmd),
                Err(e) => {
                    write!(self.lock, "error: {}", e).unwrap();
                    self.lock.flush().unwrap();
                    None
                }
            };
        }
    }
    /// Write the buffer to the terminal, and move the cursor to the bottom left
    fn _draw(&mut self) {
        fn write_ansi<P: Pixel, C: Converter>(
            viuwa: &mut Viuwa<P>,
            mut ansi: AnsiImage<ImageView<P>, C>,
            (offx, offy): (u16, u16),
        ) where
            <P as viuwa_image::Pixel>::Scalar: CompatScalar,
            <P as viuwa_image::Pixel>::Repr: CompatPixelRepr,
        {
            for (y, row) in ansi.rows_upper(viuwa.attrs, None).enumerate() {
                execute_stdout!(viuwa.lock, cursor_to(offx, offy + y as u16));
                match row {
                    AnsiRow::Full(row) => {
                        for p in row {
                            execute_stdout!(viuwa.lock, write_all(p.as_bytes()));
                        }
                    }
                    AnsiRow::Half(row) => {
                        for p in row {
                            execute_stdout!(viuwa.lock, write_all(p.as_bytes()));
                        }
                    }
                }
                execute_stdout!(viuwa.lock, attr_reset());
            }
        }
        execute_stdout!(self.lock, clear());
        let offx = (self.sz.0.saturating_sub(self.buf.width() as u16)) / 2;
        let offy = (self.sz.1.saturating_sub(div_ceil2(self.buf.height()) as u16)) / 2;
        let ansi = DynamicAnsiImage::new(unsafe { &*((&self.buf) as *const Image<P>) }.view(), self.conf.color);
        match ansi {
            DynamicAnsiImage::Color(a) => write_ansi(self, a, (offx, offy)),
            DynamicAnsiImage::Gray(a) => write_ansi(self, a, (offx, offy)),
            DynamicAnsiImage::AnsiColor(a) => write_ansi(self, a, (offx, offy)),
            DynamicAnsiImage::AnsiGray(a) => write_ansi(self, a, (offx, offy)),
        }
        #[cfg(target_os = "wasi")]
        execute_stdout!(self.lock, cursor_to(0, self.sz.1 - 1));
        execute_stdout!(self.lock, flush());
    }
    /// clear screen, print help, and quit 'q'
    fn help(&mut self) {
        execute_stdout!(self.lock, clear(), cursor_home());
        self.write_centerx(0, "Viuwa help:");
        self.write_centerxy_align_all([
            "quit                      exit the current screen",
            "help                      show this help screen",
            "refresh                   redraw the image",
            "reload                    reload the image buffer and refresh",
            "cycle_color               cycle through color modes",
            "cycle_filter              cycle through image filters",
            "cycle_color_depth         cycle through color depths",
            "cycle_color_space         cycle through color spaces",
            "set <config> <value>      set a configuration value",
            "bind <keybind> <command>  bind a keybind to a command",
            "unbind <keybind>          unbind a keybind",
        ]);
        #[cfg(target_os = "wasi")]
        execute_stdout!(self.lock, cursor_to(0, self.sz.1 - 1));
        execute_stdout!(self.lock, flush());
        self.wait_for_quit()
    }
    /// print a string centered on the x axis
    fn write_centerx<S: AsRef<str>>(&mut self, y: u16, s: S) {
        execute_stdout!(
            self.lock,
            cursor_to((self.sz.0 - s.as_ref().len() as u16) / 2, y),
            write_all(s.as_ref().as_bytes())
        );
    }
    /// print strings centered and aligned on the x axis and y axis
    fn write_centerxy_align_all<
        S: AsRef<str>,
        I: Iterator<Item = S> + ExactSizeIterator + Clone,
        C: IntoIterator<Item = S, IntoIter = I>,
    >(
        &mut self,
        s: C,
    ) {
        let s = s.into_iter();
        let len = s.len();
        if let Some(max) = s.clone().map(|x| x.as_ref().len()).max() {
            let ox = (self.sz.0 - max as u16) / 2;
            let oy = (self.sz.1 - len as u16) / 2;
            for (i, line) in s.enumerate() {
                execute_stdout!(self.lock, cursor_to(ox, oy + i as u16), write_all(line.as_ref().as_bytes()));
            }
        }
    }
    /// Reprint ANSI sequences to the terminal
    pub fn refresh(&mut self) {
        trace!("Viuwa::refresh");
        self._draw()
    }
    /// Refresh with a rebuilt buffer
    pub fn reload(&mut self) {
        trace!("Viuwa::reload");
        let dims = dimensions(self.sz, &self.conf, self.orig.dimensions());
        #[cfg(feature = "fir")]
        {
            self.buf = self.orig.fir_superscale(dims.0, dims.1, &self.conf.filter, 3);
        }
        #[cfg(not(feature = "fir"))]
        {
            self.buf = self.orig.superscale(dims.0, dims.1, &self.conf.filter, 3.);
        }
        self._draw()
    }
    fn wait_for_quit(&mut self) {
        #[cfg(not(target_os = "wasi"))]
        {
            loop {
                match crossterm::event::read().expect("failed to read event") {
                    Event::Key(e) if e.kind == KeyEventKind::Press => {
                        if e.code == KeyCode::Char(':') {
                            if let Some(cmd) = self.parse_command() {
                                match cmd {
                                    Command::Quit => break,
                                    _ => self.command(cmd),
                                }
                            }
                        } else if let Some(cmd) = self.conf.keybinds.get(&KeyBind(e)) {
                            match cmd {
                                Action::Quit => break,
                                _ => self.command(cmd.clone().into()),
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
        #[cfg(target_os = "wasi")]
        {
            use std::io::BufRead;

            use rustix::{fd::BorrowedFd, io::*};
            let stdin_raw = unsafe { BorrowedFd::borrow_raw(0) };
            let mut stdin = std::io::stdin().lock();
            let mut buf = String::new();
            loop {
                while let Ok(0) = ioctl_fionread(stdin_raw) {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                buf.clear();
                if stdin.read_line(&mut buf).expect("failed to read stdin") == 0 {
                    break;
                }
                let key = buf.trim_end_matches(['\r', '\n']);
                if key == ":" {
                    if let Some(cmd) = self.parse_command() {
                        match cmd {
                            Command::Quit => break,
                            _ => self.command(cmd),
                        }
                    }
                } else if let Some(cmd) = self.conf.keybinds.get(key) {
                    let cmd = cmd.clone();
                    if cmd == Action::Quit {
                        break;
                    } else {
                        self.command(cmd.into());
                    }
                }
            }
        }
    }
}

/// Display an image in the terminal inlined
pub fn inlined(orig: DynamicImage, conf: Config) -> Result<()> {
    trace!("inlined");
    let dims = orig.dimensions();
    let dims = (dims.0 as usize, dims.1 as usize);
    let dims = dimensions(terminal_size(&mut stdout(), &conf)?, &conf, dims);
    fn write_ansi<P: Pixel, C: Converter>(
        lock: &mut BufWriter<StdoutLock>,
        mut ansi: AnsiImage<ImageView<P>, C>,
        config: &Config,
    ) -> io::Result<()>
    where
        <P as viuwa_image::Pixel>::Scalar: CompatScalar,
        <P as viuwa_image::Pixel>::Repr: CompatPixelRepr,
    {
        for row in ansi.rows_upper(ColorAttributes::new(config.luma_correct as u32), None) {
            match row {
                AnsiRow::Full(row) => {
                    for p in row {
                        lock.write_all(p.as_bytes())?;
                    }
                }
                AnsiRow::Half(row) => {
                    for p in row {
                        lock.write_all(p.as_bytes())?;
                    }
                }
            }
            lock.attr_reset()?;
            lock.write_all(b"\n")?;
        }
        Ok(())
    }
    let mut lock = BufWriter::new(stdout().lock());
    if orig.color().has_color() {
        let orig = orig.into_rgb8();
        let orig = if cfg!(feature = "fir") {
            ImageView::from(&orig).fir_superscale(dims.0, dims.1, &conf.filter, 3)
        } else {
            ImageView::from(&orig).superscale(dims.0, dims.1, &conf.filter, 3.)
        };
        let ansi = DynamicAnsiImage::new(ImageView::from(&orig), conf.color);
        match ansi {
            DynamicAnsiImage::Color(a) => write_ansi(&mut lock, a, &conf)?,
            DynamicAnsiImage::Gray(a) => write_ansi(&mut lock, a, &conf)?,
            DynamicAnsiImage::AnsiColor(a) => write_ansi(&mut lock, a, &conf)?,
            DynamicAnsiImage::AnsiGray(a) => write_ansi(&mut lock, a, &conf)?,
        }
    } else {
        let orig = orig.into_luma8();
        let orig = {
            #[cfg(feature = "fir")]
            {
                ImageView::from(&orig).fir_superscale(dims.0, dims.1, &conf.filter, 3)
            }
            #[cfg(not(feature = "fir"))]
            {
                ImageView::from(&orig).superscale(dims.0, dims.1, &conf.filter, 3.)
            }
        };
        let ansi = DynamicAnsiImage::new(ImageView::from(&orig), conf.color);
        match ansi {
            DynamicAnsiImage::Color(a) => write_ansi(&mut lock, a, &conf)?,
            DynamicAnsiImage::Gray(a) => write_ansi(&mut lock, a, &conf)?,
            DynamicAnsiImage::AnsiColor(a) => write_ansi(&mut lock, a, &conf)?,
            DynamicAnsiImage::AnsiGray(a) => write_ansi(&mut lock, a, &conf)?,
        }
    }
    if conf.clear {
        // wait for keypress or any input
        #[cfg(not(target_os = "wasi"))]
        {
            loop {
                match crossterm::event::read().expect("failed to read event") {
                    Event::Key(e) if e.kind == KeyEventKind::Press => {
                        break;
                    }
                    _ => (),
                }
            }
        }
        #[cfg(target_os = "wasi")]
        {
            use rustix::{fd::BorrowedFd, io::*};
            let stdin_raw = unsafe { BorrowedFd::borrow_raw(0) };
            while let Ok(0) = ioctl_fionread(stdin_raw) {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        // then clear the screen
        lock.clear()?;
    }
    lock.flush()?;
    Ok(())
}

/// Create a new viuwa instance and spawn it
pub fn windowed<'a>(orig: DynamicImage, config: Config) -> Result<()> {
    trace!("windowed");
    if orig.color().has_color() {
        Viuwa::new(orig.into_rgb8(), config)?.spawn();
    } else {
        Viuwa::new(orig.into_luma8(), config)?.spawn();
    }
    Ok(())
}

/// Get the terminal size or use the default size if it is set
#[inline]
pub fn terminal_size(term: &mut impl Terminal, conf: &Config) -> Result<(u16, u16)> {
    term.size().or_else(|_| {
        if conf.default_columns.is_some() || conf.default_rows.is_some() {
            Ok((conf.default_columns.unwrap_or(1), conf.default_rows.unwrap_or(1)))
        } else {
            Err(anyhow!("Could not get terminal size"))
        }
    })
}

/// Get the dimensions of the image to be displayed in the terminal by taking into account the terminal size, the image size, and the configuration
#[inline]
pub fn dimensions(term_sz: (u16, u16), conf: &Config, img_sz: (usize, usize)) -> (usize, usize) {
    let fit = (term_sz.0 as usize, term_sz.1 as usize * 2);
    let fill = viuwa_image::fill_dimensions(img_sz, fit);
    match (conf.width, conf.height) {
        (Dimension::Fit, Dimension::Fit) => fit,
        (Dimension::Fit, Dimension::Fill) => (fit.0, fill.1),
        (Dimension::Fit, Dimension::Limit(h)) => (fit.0, h as usize),
        (Dimension::Fill, Dimension::Fit) => (fill.0, fit.1),
        (Dimension::Fill, Dimension::Fill) => fill,
        (Dimension::Fill, Dimension::Limit(h)) => (fill.0, h as usize),
        (Dimension::Limit(w), Dimension::Fit) => (w as usize, fit.1),
        (Dimension::Limit(w), Dimension::Fill) => (w as usize, fill.1),
        (Dimension::Limit(w), Dimension::Limit(h)) => (w as usize, h as usize),
    }
}

/// Very basic check to see if terminal supports ansi
#[cfg(not(windows))]
pub fn supports_ansi() -> bool {
    use std::env::var;
    var("TERM").map_or(false, |term| term != "dumb")
}
/// Very basic check to see if terminal supports ansi, and enables Virtual Terminal Processing on Windows
#[cfg(windows)]
pub fn supports_ansi() -> bool { crossterm::ansi_support::supports_ansi() }
/// Warnings for ansi support and windows (disabled on wasm because we can't really check)
#[cfg(target_family = "wasm")]
#[inline(always)]
fn warnings() -> Result<(), ()> { Ok(()) }
/// Warnings for ansi support and windows
#[cfg(not(target_family = "wasm"))]
fn warnings() -> Result<(), ()> {
    let is_ansi = supports_ansi();
    if !is_ansi {
        warn!("Could not verify that terminal supports ansi. Continue? [Y/n] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_ascii_lowercase();
        if input.starts_with("n") {
            return Err(());
        }
    }
    Ok(())
}

/// Default main function for viuwa
pub fn main() -> Result<()> {
    // this should be compatible with almost all platforms
    if !::is_terminal::IsTerminal::is_terminal(&std::io::stdout()) {
        warn!("We cannot guarantee that viuwa will work as intended in a file or pipe");
    }
    #[cfg(feature = "debug")]
    {
        debug!("start", "features:\t");
        #[cfg(feature = "env")]
        eprint!("env, ");
        #[cfg(feature = "rayon")]
        eprint!("rayon, ");
        #[cfg(feature = "fir")]
        eprint!("fir, ");
        #[cfg(feature = "trace")]
        eprint!("trace, ");
        eprintln!("debug");
    }
    trace!("main");
    let config = Config::new();
    debug!("main", "generated config: {:#?}", config);
    LOG_LEVEL.with(|cell| cell.set(config.log));
    if warnings().is_err() {
        return Ok(());
    }
    let orig = {
        info!("loading image...");
        image::io::Reader::open(&config.image)?
            .with_guessed_format()?
            .decode()
            .context("Failed to load image, the file extension may be incorrect")?
    };
    // Any errors from here on out are likely to not be the users direct fault, so we can ask for a bug report
    #[cfg(not(target_os = "wasi"))]
    human_panic::setup_panic!();
    // unwraps so that we can use panic to report a bug if this fails, (better than opaque errors)
    // most likely due to std::io::stdout() write failing
    if !config.inline {
        windowed(orig, config).expect("Failed to display image windowed");
    } else {
        inlined(orig, config).expect("Failed to display image inlined")
    }
    Ok(())
}
