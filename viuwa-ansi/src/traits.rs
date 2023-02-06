use std::{
    fmt::Display,
    io::{self, Result, Write},
};

use num_traits::Unsigned;

use crate::{consts::*, statics};

/// Terminal ANSI writes
pub trait Terminal: Write + Sized {
    /// Clear the screen and the buffer
    #[inline]
    fn clear(&mut self) -> Result<()> { self.clear_screen().and_then(|_| self.clear_buffer()) }
    #[inline]
    fn clear_buffer(&mut self) -> Result<()> { self.write_all(CLEAR_BUFFER.as_bytes()) }
    #[inline]
    fn clear_screen(&mut self) -> Result<()> { self.write_all(CLEAR_SCREEN.as_bytes()) }
    #[inline]
    fn clear_line(&mut self) -> Result<()> { self.write_all(CLEAR_LINE.as_bytes()) }
    #[inline]
    fn clear_line_to_end(&mut self) -> Result<()> { self.write_all(CLEAR_LINE_TO_END.as_bytes()) }
    #[inline]
    fn clear_line_to_start(&mut self) -> Result<()> { self.write_all(CLEAR_LINE_TO_START.as_bytes()) }
    #[inline]
    fn clear_screen_to_end(&mut self) -> Result<()> { self.write_all(CLEAR_SCREEN_TO_END.as_bytes()) }
    #[inline]
    fn clear_screen_to_start(&mut self) -> Result<()> { self.write_all(CLEAR_SCREEN_TO_START.as_bytes()) }
    #[inline]
    /// does not work on windows
    fn reset(&mut self) -> Result<()> { self.write_all(RESET.as_bytes()) }
    #[inline]
    fn soft_reset(&mut self) -> Result<()> { self.write_all(SOFT_RESET.as_bytes()) }
    #[inline]
    fn enter_alt_screen(&mut self) -> Result<()> { self.write_all(ENTER_ALT_SCREEN.as_bytes()) }
    #[inline]
    fn exit_alt_screen(&mut self) -> Result<()> { self.write_all(EXIT_ALT_SCREEN.as_bytes()) }
    #[inline]
    fn enable_line_wrap(&mut self) -> Result<()> { self.write_all(ENABLE_LINE_WRAP.as_bytes()) }
    #[inline]
    fn disable_line_wrap(&mut self) -> Result<()> { self.write_all(DISABLE_LINE_WRAP.as_bytes()) }
    #[inline]
    fn enable_raw_mode(&mut self) -> Result<()> {
        #[cfg(target_family = "wasm")]
        return {
            Ok(()) //REVIEW: Do we fail successfully or fail unsuccessfully?
                   // Err(io::Error::from(io::ErrorKind::PermissionDenied))
                   // There is literally no way to do this in wasm
        };
        #[cfg(not(target_family = "wasm"))]
        return ::crossterm::terminal::enable_raw_mode();
    }
    #[inline]
    fn disable_raw_mode(&mut self) -> Result<()> {
        #[cfg(target_family = "wasm")]
        {
            return {
                Ok(()) //REVIEW: Do we fail successfully or fail unsuccessfully?
                       // Err(io::Error::from(io::ErrorKind::PermissionDenied))
                       // There is literally no way to do this in wasm
            };
        }
        #[cfg(not(target_family = "wasm"))]
        return ::crossterm::terminal::disable_raw_mode();
    }
    /// Set the window title using ansi escape codes
    #[inline]
    fn set_title<T: ::std::fmt::Display>(&mut self, title: &T) -> Result<()> { write!(self, osc!("0;", st!("{}")), title) }
    #[inline]
    /// Resize the window using ansi escape codes
    fn resize(&mut self, width: u16, height: u16) -> Result<()> { write!(self, csi!("8;{};{}t"), height, width) }
    /// Attempt to read the terminal size in characters
    #[inline]
    fn size(&mut self) -> Result<(u16, u16)> {
        #[cfg(not(target_family = "wasm"))]
        return ::crossterm::terminal::size();
        #[cfg(target_family = "wasm")]
        {
            use std::io::Read;
            #[cfg(feature = "wasi-request-size")]
            fn _request_size(t: &mut impl Terminal) -> Result<(u16, u16)> {
                eprintln!("requesting size, please enter on response...");
                t.cursor_save()?;
                t.write_all(b"\x1b[4096;4096H")?;
                t.cursor_report_position()?;
                t.cursor_restore()?;
                t.flush()?;
                let mut buf = [0; 11];
                let mut res = [None; 2];
                if matches!(io::stdin().read(&mut buf), Ok(n) if n >= 6) {
                    let buf = buf.into_iter().filter(|&b| b == b';' || b.is_ascii_digit()).collect::<Vec<_>>();
                    for (b, r) in buf.splitn(2, |&b| b == b';').zip(res.iter_mut()) {
                        *r = unsafe { std::str::from_utf8_unchecked(b) }.parse::<u16>().ok();
                    }
                };
                if let [Some(w), Some(h)] = res {
                    Ok((w, h))
                } else {
                    Err(io::Error::from(io::ErrorKind::Other))
                }
            }
            #[cfg(feature = "wasi-request-size")]
            return {
                if let (Some(w), Some(h)) = (
                    std::env::var("COLUMNS").ok().and_then(|h| h.parse::<u16>().ok()),
                    std::env::var("LINES").ok().and_then(|h| h.parse::<u16>().ok()),
                ) {
                    Ok((w, h))
                } else if let Ok(v) = _request_size(self) {
                    Ok(v)
                } else {
                    Err(io::Error::from(io::ErrorKind::Other))
                }
            };
            #[cfg(not(feature = "wasi-request-size"))]
            return {
                if let (Some(w), Some(h)) = (
                    std::env::var("COLUMNS").ok().and_then(|h| h.parse::<u16>().ok()),
                    std::env::var("LINES").ok().and_then(|h| h.parse::<u16>().ok()),
                ) {
                    Ok((w, h))
                } else {
                    Err(io::Error::from(io::ErrorKind::Other))
                }
            };
        }
    }
    #[inline]
    fn cursor_hide(&mut self) -> Result<()> { self.write_all(HIDE_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_show(&mut self) -> Result<()> { self.write_all(SHOW_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_save(&mut self) -> Result<()> { self.write_all(SAVE_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_restore(&mut self) -> Result<()> { self.write_all(RESTORE_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_report_position(&mut self) -> Result<()> { self.write_all(REPORT_CURSOR_POSITION.as_bytes()) }
    #[inline]
    fn cursor_next_line(&mut self) -> Result<()> { self.write_all(CURSOR_NEXT_LINE.as_bytes()) }
    #[inline]
    fn cursor_prev_line(&mut self) -> Result<()> { self.write_all(CURSOR_PREV_LINE.as_bytes()) }
    #[inline]
    fn cursor_home(&mut self) -> Result<()> { self.write_all(CURSOR_HOME.as_bytes()) }
    #[inline]
    fn cursor_to(&mut self, x: u16, y: u16) -> Result<()> { write!(self, csi!("{};{}H"), y + 1, x + 1) }
    #[inline]
    fn cursor_to_col(&mut self, x: u16) -> Result<()> { write!(self, csi!("{}G"), x + 1) }
    #[inline]
    fn cursor_up(&mut self, n: u16) -> Result<()> { write!(self, csi!("{}A"), n) }
    #[inline]
    fn cursor_down(&mut self, n: u16) -> Result<()> { write!(self, csi!("{}B"), n) }
    #[inline]
    fn cursor_forward(&mut self, n: u16) -> Result<()> { write!(self, csi!("{}C"), n) }
    #[inline]
    fn cursor_backward(&mut self, n: u16) -> Result<()> { write!(self, csi!("{}D"), n) }
    #[inline]
    fn cursor_next_lines(&mut self, n: u16) -> Result<()> { write!(self, csi!("{}E"), n) }
    #[inline]
    fn cursor_prev_lines(&mut self, n: u16) -> Result<()> { write!(self, csi!("{}F"), n) }
    #[inline]
    fn attr_reset(&mut self) -> Result<()> { self.write_all(SGR_DEFAULT.as_bytes()) }
    #[inline]
    fn write_iter<'a, Item: AsRef<[u8]> + 'a, C: IntoIterator<Item = &'a Item>>(&mut self, c: C) -> Result<()> {
        for s in c {
            self.write_all(s.as_ref())?;
        }
        Ok(())
    }
}
impl Terminal for io::Stdout {}
impl<'a> Terminal for io::StdoutLock<'a> {}
impl Terminal for io::BufWriter<io::Stdout> {}
impl<'a> Terminal for io::BufWriter<io::StdoutLock<'a>> {}
impl Terminal for io::Stderr {}
impl<'a> Terminal for io::StderrLock<'a> {}
impl Terminal for io::BufWriter<io::Stderr> {}
