use crate::BoxResult;
pub mod ansi;
mod vimage;
pub use vimage::ViuwaImage;

use std::io::{self, stdin, stdout, Read, StdoutLock, Write};

use image::{imageops::FilterType, DynamicImage};

use self::ansi::TerminalImpl;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutFormat {
        AnsiRgb,
        Ansi256,
        AnsiGrey,
        // #[cfg(feature = "iterm")]
        // Iterm,
        // #[cfg(feature = "sixel")]
        // Sixel,
}
impl OutFormat {
        #[cfg(not(any(feature = "iterm", feature = "sixel")))]
        pub fn cycle(&self) -> OutFormat {
                match self {
                        OutFormat::AnsiRgb => OutFormat::Ansi256,
                        OutFormat::Ansi256 => OutFormat::AnsiGrey,
                        OutFormat::AnsiGrey => OutFormat::AnsiRgb,
                }
        }
        // #[cfg(all(feature = "iterm", feature = "sixel"))]
        // pub fn cycle(&self) -> OutFormat {
        //         match self {
        //                 OutFormat::AnsiRgb => OutFormat::Ansi256,
        //                 OutFormat::Ansi256 => OutFormat::AnsiGrey,
        //                 OutFormat::AnsiGrey => OutFormat::Iterm,
        //                 OutFormat::Iterm => OutFormat::Sixel,
        //                 OutFormat::Sixel => OutFormat::AnsiRgb,
        //         }
        // }
        // #[cfg(all(feature = "iterm", not(feature = "sixel")))]
        // pub fn cycle(&self) -> OutFormat {
        //         match self {
        //                 OutFormat::AnsiRgb => OutFormat::Ansi256,
        //                 OutFormat::Ansi256 => OutFormat::AnsiGrey,
        //                 OutFormat::AnsiGrey => OutFormat::Iterm,
        //                 OutFormat::Iterm => OutFormat::AnsiRgb,
        //         }
        // }
        // #[cfg(all(not(feature = "iterm"), feature = "sixel"))]
        // pub fn cycle(&self) -> OutFormat {
        //         match self {
        //                 OutFormat::AnsiRgb => OutFormat::Ansi256,
        //                 OutFormat::Ansi256 => OutFormat::AnsiGrey,
        //                 OutFormat::AnsiGrey => OutFormat::Sixel,
        //                 OutFormat::Sixel => OutFormat::AnsiRgb,
        //         }
        // }
}

pub struct Viuwa<'a> {
        pub orig: DynamicImage,
        pub buf: Vec<String>,
        pub filter: FilterType,
        pub format: OutFormat,
        pub term_size: (u16, u16),
        pub lock: StdoutLock<'a>,
}

impl<'a> Viuwa<'a> {
        pub fn new(orig: DynamicImage, filter: FilterType, format: OutFormat) -> BoxResult<Self> {
                let mut lock = stdout().lock();
                let term_size = lock.size()?;
                let orig = if orig.color().has_color() {
                        DynamicImage::ImageRgb8(orig.into_rgb8())
                } else {
                        DynamicImage::ImageLuma8(orig.into_luma8())
                };
                let buf = ViuwaImage::new(orig.resize(term_size.0 as u32, term_size.1 as u32 * 2, filter), format)
                        .to_ansi_window(term_size);
                Ok(Self {
                        orig,
                        buf,
                        filter,
                        format,
                        term_size,
                        lock,
                })
        }
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
                                        KeyCode::Char('F') => {
                                                self._cycle_filter();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('f') => {
                                                self._cycle_format();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('1') => {
                                                self.format = OutFormat::AnsiRgb;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('2') => {
                                                self.format = OutFormat::Ansi256;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('3') => {
                                                self.format = OutFormat::AnsiGrey;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        // #[cfg(feature = "iterm")]
                                        // KeyCode::Char('4') => {
                                        //         self.format = OutFormat::Iterm;
                                        //         self._rebuild_buf();
                                        //         self._draw()?;
                                        // }
                                        // #[cfg(feature = "sixel")]
                                        // KeyCode::Char('5') => {
                                        //         self.format = OutFormat::Sixel;
                                        //         self._rebuild_buf();
                                        //         self._draw()?;
                                        // }
                                        KeyCode::Char('!') => {
                                                self.filter = FilterType::Nearest;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('@') => {
                                                self.filter = FilterType::Triangle;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('#') => {
                                                self.filter = FilterType::CatmullRom;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('$') => {
                                                self.filter = FilterType::Gaussian;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('%') => {
                                                self.filter = FilterType::Lanczos3;
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
        #[cfg(not(any(unix, windows)))]
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
                self._draw()?;
                let mut buf = [0; 1];
                loop {
                        stdin().read_exact(&mut buf)?;
                        match buf[0] {
                                b'q' => break,
                                b'r' => {
                                        let term_size = self.lock.size()?;
                                        self._resize(term_size.0, term_size.1);
                                        self._draw()?;
                                }
                                b'h' => {
                                        self._help()?;
                                        self._draw()?;
                                }
                                b'f' => {
                                        self._cycle_format();
                                        self._draw()?;
                                }
                                b'F' => {
                                        self._cycle_filter();
                                        self._draw()?;
                                }
                                b'1' => {
                                        self.format = OutFormat::AnsiRgb;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'2' => {
                                        self.format = OutFormat::Ansi256;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'3' => {
                                        self.format = OutFormat::AnsiGrey;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'!' => {
                                        self.filter = FilterType::Nearest;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'@' => {
                                        self.filter = FilterType::Triangle;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'#' => {
                                        self.filter = FilterType::CatmullRom;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'$' => {
                                        self.filter = FilterType::Gaussian;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'%' => {
                                        self.filter = FilterType::Lanczos3;
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
        fn _draw(&mut self) -> BoxResult<()> {
                self.lock.clear_screen()?;
                // let mut print_queue = Arc::new(Mutex::new(VecDeque::with_capacity(self.px_size.1 as usize)));
                for line in self.buf.iter() {
                        self.lock.write_all(line.as_bytes())?;
                }
                self.lock.flush()?;
                Ok(())
        }
        /// clear screen , print help, and quit 'q'
        fn _help(&mut self) -> BoxResult<()> {
                self.lock
                        .write_all([ansi::term::CLEAR_SCREEN, ansi::cursor::HOME].concat().as_bytes())?;
                self._write_centered(0, "Viuwa interative help: ")?;
                self._write_centered(1, "q to quit")?;
                self._write_centered(2, "r to redraw")?;
                self._write_centered(3, "h to print help")?;
                self._write_centered(4, "f to cycle output format")?;
                self._write_centered(5, "Shift + f to cycle image filter")?;
                self._write_centered(6, "Shift + 1-5 to set image filter")?;
                self._write_centered(7, "1-5 to set output format")?;
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
        fn _handle_resize(&mut self, w: u16, h: u16) {
                let nsz = (w + 1, h + 1);
                if nsz != self.term_size {
                        self.term_size = nsz;
                        self._rebuild_buf();
                }
        }
        pub fn inline(orig: DynamicImage, filter: FilterType, format: OutFormat, size: (u16, u16)) -> BoxResult<()> {
                let orig = if orig.color().has_color() {
                        DynamicImage::ImageRgb8(orig.into_rgb8())
                } else {
                        DynamicImage::ImageLuma8(orig.into_luma8())
                };
                let buf = ViuwaImage::new(orig.resize(size.0 as u32, size.1 as u32 * 2, filter), format).to_ansi_inline();
                let mut lock = stdout().lock();
                for line in buf.iter() {
                        lock.write_all(line.as_bytes())?;
                }
                lock.flush()?;
                Ok(())
        }
        fn _write_centered(&mut self, y: u16, s: &str) -> io::Result<()> {
                self.lock.write_all(
                        [&ansi::cursor::to((self.term_size.0 - s.len() as u16) / 2, y), s]
                                .concat()
                                .as_bytes(),
                )?;
                Ok(())
        }
        fn _cycle_filter(&mut self) {
                self.filter = match self.filter {
                        FilterType::Nearest => FilterType::Triangle,
                        FilterType::Triangle => FilterType::CatmullRom,
                        FilterType::CatmullRom => FilterType::Gaussian,
                        FilterType::Gaussian => FilterType::Lanczos3,
                        FilterType::Lanczos3 => FilterType::Nearest,
                };
                self._rebuild_buf();
        }
        fn _cycle_format(&mut self) {
                self.format = self.format.cycle();
                self._rebuild_buf();
        }
        fn _rebuild_buf(&mut self) {
                self.buf = ViuwaImage::new(
                        self.orig
                                .resize(self.term_size.0 as u32, self.term_size.1 as u32 * 2, self.filter),
                        self.format,
                )
                .to_ansi_window(self.term_size);
        }
        // pub fn sixel(orig: DynamicImage, filter: FilterType, format: OutFormat, size: (u16, u16)) -> BoxResult<()> {
        //         let orig = if orig.color().has_color() {
        //                 DynamicImage::ImageRgb8(orig.into_rgb8())
        //         } else {
        //                 DynamicImage::ImageLuma8(orig.into_luma8())
        //         };
        //         let buf = ViuwaImage::new(orig.resize(size.0 as u32, size.1 as u32 * 2, filter), format).to_sixel();
        //         let mut lock = stdout().lock();
        //         lock.write_all(buf.as_bytes())?;
        //         lock.flush()?;
        //         Ok(())
        // }
}
