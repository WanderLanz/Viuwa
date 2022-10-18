use crate::{Args, BoxResult};
pub mod ansi;
mod viuwa_image;

use std::io::{self, stdin, stdout, Read, StdoutLock, Write};

use image::{imageops::FilterType, DynamicImage};

use self::{ansi::TerminalImpl, viuwa_image::AnsiImage};

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
        pub fn cycle(&self) -> ColorType {
                match self {
                        ColorType::Color => ColorType::Color256,
                        ColorType::Color256 => ColorType::Gray,
                        ColorType::Gray => ColorType::Gray256,
                        ColorType::Gray256 => ColorType::Color,
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
        pub orig: DynamicImage,
        pub buf: Vec<String>,
        pub size: (u16, u16),
        pub lock: StdoutLock<'a>,
        pub args: Args,
}

impl<'a> Viuwa<'a> {
        /// Create a new viuwa instance
        pub fn new(orig: DynamicImage, args: Args) -> BoxResult<Self> {
                let mut lock = stdout().lock();
                let size = lock.size(args.quiet)?;
                let buf = orig.resize(size.0 as u32, size.1 as u32 * 2, args.filter).into_ansi_windowed(
                        args.color,
                        ColorAttributes::new(args.luma_correct),
                        size,
                );
                Ok(Self {
                        orig,
                        buf,
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
                                                self.args.filter = FilterType::Nearest;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('@') => {
                                                self.args.filter = FilterType::Triangle;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('#') => {
                                                self.args.filter = FilterType::CatmullRom;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('$') => {
                                                self.args.filter = FilterType::Gaussian;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('%') => {
                                                self.args.filter = FilterType::Lanczos3;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        _ => {}
                                },
                                Event::Resize(w, h) => {
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
                                        self.args.filter = FilterType::Nearest;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'@' => {
                                        self.args.filter = FilterType::Triangle;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'#' => {
                                        self.args.filter = FilterType::CatmullRom;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'$' => {
                                        self.args.filter = FilterType::Gaussian;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'%' => {
                                        self.args.filter = FilterType::Lanczos3;
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
                self.lock.clear_screen()?;
                // let mut print_queue = Arc::new(Mutex::new(VecDeque::with_capacity(self.px_size.1 as usize)));
                for line in self.buf.iter() {
                        self.lock.write_all(line.as_bytes())?;
                }
                self.lock.write_all(ansi::cursor::to(0, self.size.1).as_bytes())?;
                self.lock.flush()?;
                Ok(())
        }
        /// clear screen, print help, and quit 'q'
        fn _help(&mut self) -> BoxResult<()> {
                self.lock
                        .write_all([ansi::term::CLEAR_SCREEN, ansi::cursor::HOME].concat().as_bytes())?;
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
                self.lock.write_all(ansi::cursor::to(0, self.size.1).as_bytes())?;
                self.lock.flush()?;
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
        /// handle resize event
        fn _handle_resize(&mut self, w: u16, h: u16) {
                let nsz = (w + 1, h + 1);
                if nsz != self.size {
                        self.size = nsz;
                        self._rebuild_buf();
                }
        }
        /// Print ANSI image to stdout without attempting to use alternate screen buffer or other fancy stuff
        pub fn inline(orig: DynamicImage, args: Args) -> BoxResult<()> {
                let size = match (args.width, args.height) {
                        (None, None) => stdout().size(args.quiet)?,
                        (None, Some(h)) => (crate::MAX_COLS, h),
                        (Some(w), None) => (w, crate::MAX_ROWS),
                        (Some(w), Some(h)) => (w, h),
                };
                let buf = orig
                        .resize(size.0 as u32, size.1 as u32 * 2, args.filter)
                        .into_ansi_inline(args.color, ColorAttributes::new(args.luma_correct));
                let mut lock = stdout().lock();
                for line in buf.iter() {
                        lock.write_all(line.as_bytes())?;
                }
                lock.flush()?;
                Ok(())
        }
        /// print a string centered on the x axis
        fn _write_centerx(&mut self, y: u16, s: &str) -> io::Result<()> {
                self.lock.write_all(
                        [&ansi::cursor::to((self.size.0 - s.len() as u16) / 2, y), s]
                                .concat()
                                .as_bytes(),
                )?;
                Ok(())
        }
        /// print strings centered and aligned on the x axis and y axis
        fn _write_centerxy_align_all(&mut self, s: &Vec<&str>) -> BoxResult<()> {
                if let Some(max) = s.into_iter().map(|x| x.len()).max() {
                        let ox = (self.size.0 - max as u16) / 2;
                        let oy = (self.size.1 - s.len() as u16) / 2;
                        for (i, line) in s.into_iter().enumerate() {
                                self.lock
                                        .write_all([&ansi::cursor::to(ox, oy + i as u16), *line].concat().as_bytes())?;
                        }
                        Ok(())
                } else {
                        Err("No strings to write".into())
                }
        }
        /// Cycle through filter types, and rebuild buffer
        fn _cycle_filter(&mut self) {
                self.args.filter = match self.args.filter {
                        FilterType::Nearest => FilterType::Triangle,
                        FilterType::Triangle => FilterType::CatmullRom,
                        FilterType::CatmullRom => FilterType::Gaussian,
                        FilterType::Gaussian => FilterType::Lanczos3,
                        FilterType::Lanczos3 => FilterType::Nearest,
                };
                self._rebuild_buf();
        }
        /// Cycle through output formats, and rebuild buffer
        fn _cycle_color(&mut self) {
                self.args.color = self.args.color.cycle();
                self._rebuild_buf();
        }
        /// Rebuild the buffer with the current image, filter, and format
        fn _rebuild_buf(&mut self) {
                self.buf = self
                        .orig
                        .resize(self.size.0 as u32, self.size.1 as u32 * 2, self.args.filter)
                        .into_ansi_windowed(self.args.color, ColorAttributes::new(self.args.luma_correct), self.size);
        }
}
