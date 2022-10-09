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
use crate::BoxResult;
use std::io::{self, Write};
#[cfg(not(any(unix, windows)))]
use std::io::{stdin, Read};
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
        const MAX: Coord = Coord { x: 0x7FFF, y: 0x7FFF };
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

pub trait TerminalImpl
where
        Self: Write + Sized,
{
        fn clear_buffer(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_BUFFER.as_bytes()) }
        fn clear_screen(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_SCREEN.as_bytes()) }
        fn clear_line(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_LINE.as_bytes()) }
        fn clear_line_to_end(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_LINE_TO_END.as_bytes()) }
        fn clear_line_to_start(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_LINE_TO_START.as_bytes()) }
        fn clear_screen_to_end(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_SCREEN_TO_END.as_bytes()) }
        fn clear_screen_to_start(&mut self) -> io::Result<()> { self.write_all(term::CLEAR_SCREEN_TO_START.as_bytes()) }
        /// !windows
        fn reset(&mut self) -> io::Result<()> { self.write_all(term::RESET.as_bytes()) }
        fn soft_reset(&mut self) -> io::Result<()> { self.write_all(term::SOFT_RESET.as_bytes()) }
        fn enter_alt_screen(&mut self) -> io::Result<()> { self.write_all(term::ENTER_ALT_SCREEN.as_bytes()) }
        fn exit_alt_screen(&mut self) -> io::Result<()> { self.write_all(term::EXIT_ALT_SCREEN.as_bytes()) }
        fn enable_line_wrap(&mut self) -> io::Result<()> { self.write_all(term::ENABLE_LINE_WRAP.as_bytes()) }
        fn disable_line_wrap(&mut self) -> io::Result<()> { self.write_all(term::DISABLE_LINE_WRAP.as_bytes()) }
        #[cfg(any(windows, unix))]
        #[inline]
        fn enable_raw_mode(&mut self) -> io::Result<()> { ::crossterm::terminal::enable_raw_mode() }
        #[cfg(not(any(windows, unix)))]
        fn enable_raw_mode(&mut self) -> io::Result<()> { self.write_all(term::ENABLE_RAW_MODE.as_bytes()) }
        #[cfg(any(windows, unix))]
        #[inline]
        fn disable_raw_mode(&mut self) -> io::Result<()> { ::crossterm::terminal::disable_raw_mode() }
        #[cfg(not(any(windows, unix)))]
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
        fn size(&mut self) -> io::Result<(u16, u16)> { ::crossterm::terminal::size() }
        /// Attempt to read the terminal size in characters using only ANSI escape sequences
        ///   
        /// It is not guaranteed to work, although more universal than a direct x-term style ANSI window size request "\x1B[18t".  
        /// Works best in raw alternate screen mode.
        /// relies on the user to press enter because we cannot read stdout.
        /// WARNING: this is a blocking call
        #[cfg(not(any(windows, unix)))]
        fn size(&mut self) -> io::Result<(u16, u16)> {
                self.write_all(["Requesting terminal size report, please press enter when a report appears (e.g. \"^[[40;132R\")\n",term::SAVE_CURSOR,&cursor::to(Coord::MAX.x, Coord::MAX.y),term::REPORT_CURSOR_POSITION,term::RESTORE_CURSOR].concat().as_bytes())?;
                self.flush()?;
                let mut buf = [0; 1];
                let mut s = Vec::<u8>::with_capacity(10);
                loop {
                        stdin().read_exact(&mut buf)?;
                        match buf[0] {
                                b'\x1B' => {
                                        stdin().read_exact(&mut buf)?;
                                        if buf[0] != b'[' {
                                                continue;
                                        }
                                        loop {
                                                stdin().read_exact(&mut buf)?;
                                                if buf[0] == b'R' || buf[0] == b'\0' {
                                                        break;
                                                }
                                                s.push(buf[0]);
                                        }
                                        break;
                                }
                                b'\0' | b'\n' => break,
                                _ => (),
                        }
                }
                let s = String::from_utf8(s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                if let Ok(coord) = Coord::try_from_report(&s) {
                        Ok((coord.x + 1, coord.y + 1))
                } else {
                        Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "failed to read terminal size using ANSI escape sequences",
                        ))
                }
        }
        fn cursor(&mut self) -> Cursor<Self> { Cursor::new(self) }
        fn cursor_hide(&mut self) -> io::Result<()> { self.write_all(term::HIDE_CURSOR.as_bytes()) }
        fn cursor_show(&mut self) -> io::Result<()> { self.write_all(term::SHOW_CURSOR.as_bytes()) }
        fn cursor_save(&mut self) -> io::Result<()> { self.write_all(term::SAVE_CURSOR.as_bytes()) }
        fn cursor_restore(&mut self) -> io::Result<()> { self.write_all(term::RESTORE_CURSOR.as_bytes()) }
        fn cursor_report_position(&mut self) -> io::Result<()> { self.write_all(term::REPORT_CURSOR_POSITION.as_bytes()) }

        // fn start_sixel(&mut self) -> io::Result<()> { self.write_all(term::START_SIXEL.as_bytes()) }
}
impl<'a> TerminalImpl for io::StdoutLock<'a> {}
impl TerminalImpl for io::Stdout {}

pub struct Cursor<'a, T: Write>(&'a mut T);
impl<'a, T: Write> Cursor<'a, T> {
        pub fn new(term: &'a mut T) -> Self { Self(term) }
        pub fn next_line(&mut self) -> io::Result<()> { self.0.write_all(cursor::NEXT_LINE.as_bytes()) }
        pub fn prev_line(&mut self) -> io::Result<()> { self.0.write_all(cursor::PREV_LINE.as_bytes()) }
        pub fn home(&mut self) -> io::Result<()> { self.0.write_all(cursor::HOME.as_bytes()) }
        pub fn to(&mut self, x: u16, y: u16) -> io::Result<()> { self.0.write_all(cursor::to(x, y).as_bytes()) }
        pub fn to_col(&mut self, x: u16) -> io::Result<()> { self.0.write_all(cursor::to_col(x).as_bytes()) }
        pub fn up(&mut self, n: u16) -> io::Result<()> { self.0.write_all(cursor::up(n).as_bytes()) }
        pub fn down(&mut self, n: u16) -> io::Result<()> { self.0.write_all(cursor::down(n).as_bytes()) }
        pub fn foward(&mut self, n: u16) -> io::Result<()> { self.0.write_all(cursor::foward(n).as_bytes()) }
        pub fn backward(&mut self, n: u16) -> io::Result<()> { self.0.write_all(cursor::backward(n).as_bytes()) }
        pub fn next_lines(&mut self, n: u16) -> io::Result<()> { self.0.write_all(cursor::next_line(n).as_bytes()) }
        pub fn prev_lines(&mut self, n: u16) -> io::Result<()> { self.0.write_all(cursor::prev_line(n).as_bytes()) }
}

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
        pub fn to(x: u16, y: u16) -> String { format!(csi!("{};{}H"), y + 1, x + 1) }
        pub fn to_col(x: u16) -> String { format!(csi!("{}G"), x) }
        pub fn up(n: u16) -> String { format!(csi!("{}A"), n) }
        pub fn down(n: u16) -> String { format!(csi!("{}B"), n) }
        pub fn foward(n: u16) -> String { format!(csi!("{}C"), n) }
        pub fn backward(n: u16) -> String { format!(csi!("{}D"), n) }
        pub fn next_line(n: u16) -> String { format!(csi!("{}E"), n) }
        pub fn prev_line(n: u16) -> String { format!(csi!("{}F"), n) }
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
