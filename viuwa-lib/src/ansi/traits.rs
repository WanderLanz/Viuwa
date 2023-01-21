use super::*;

/// Add terminal ANSI writes to a impl Write
pub trait TerminalImpl: io::Write + Sized {
    #[inline]
    fn clear(&mut self) -> io::Result<()> { self.clear_screen().and_then(|_| self.clear_buffer()) }
    #[inline]
    fn clear_buffer(&mut self) -> io::Result<()> { self.write_all(CLEAR_BUFFER.as_bytes()) }
    #[inline]
    fn clear_screen(&mut self) -> io::Result<()> { self.write_all(CLEAR_SCREEN.as_bytes()) }
    #[inline]
    fn clear_line(&mut self) -> io::Result<()> { self.write_all(CLEAR_LINE.as_bytes()) }
    #[inline]
    fn clear_line_to_end(&mut self) -> io::Result<()> { self.write_all(CLEAR_LINE_TO_END.as_bytes()) }
    #[inline]
    fn clear_line_to_start(&mut self) -> io::Result<()> { self.write_all(CLEAR_LINE_TO_START.as_bytes()) }
    #[inline]
    fn clear_screen_to_end(&mut self) -> io::Result<()> { self.write_all(CLEAR_SCREEN_TO_END.as_bytes()) }
    #[inline]
    fn clear_screen_to_start(&mut self) -> io::Result<()> { self.write_all(CLEAR_SCREEN_TO_START.as_bytes()) }
    #[inline]
    /// does not work on windows
    fn reset(&mut self) -> io::Result<()> { self.write_all(RESET.as_bytes()) }
    #[inline]
    fn soft_reset(&mut self) -> io::Result<()> { self.write_all(SOFT_RESET.as_bytes()) }
    #[inline]
    fn enter_alt_screen(&mut self) -> io::Result<()> { self.write_all(ENTER_ALT_SCREEN.as_bytes()) }
    #[inline]
    fn exit_alt_screen(&mut self) -> io::Result<()> { self.write_all(EXIT_ALT_SCREEN.as_bytes()) }
    #[inline]
    fn enable_line_wrap(&mut self) -> io::Result<()> { self.write_all(ENABLE_LINE_WRAP.as_bytes()) }
    #[inline]
    fn disable_line_wrap(&mut self) -> io::Result<()> { self.write_all(DISABLE_LINE_WRAP.as_bytes()) }
    #[cfg(all(not(target_family = "wasm"), feature = "crossterm"))]
    #[inline]
    fn enable_raw_mode(&mut self) -> io::Result<()> { ::crossterm::terminal::enable_raw_mode() }
    #[cfg(any(target_family = "wasm", not(feature = "crossterm")))]
    fn enable_raw_mode(&mut self) -> io::Result<()> { self.write_all(ENABLE_RAW_MODE.as_bytes()) }
    #[cfg(all(not(target_family = "wasm"), feature = "crossterm"))]
    #[inline]
    fn disable_raw_mode(&mut self) -> io::Result<()> { ::crossterm::terminal::disable_raw_mode() }
    #[cfg(any(target_family = "wasm", not(feature = "crossterm")))]
    #[inline]
    fn disable_raw_mode(&mut self) -> io::Result<()> { self.write_all(DISABLE_RAW_MODE.as_bytes()) }

    /// Set the window title using ansi escape codes
    fn set_title<T: ::std::fmt::Display>(&mut self, title: &T) -> io::Result<()> {
        write!(self, osc!("0;", st!("{}")), title)
    }
    /// Resize the window using ansi escape codes
    fn resize(&mut self, width: u16, height: u16) -> io::Result<()> { write!(self, csi!("8;{};{}t"), height, width) }
    /// Attempt to read the terminal size in characters
    #[cfg(all(not(target_family = "wasm"), feature = "crossterm"))]
    #[inline]
    fn size(&mut self) -> io::Result<(u16, u16)> { ::crossterm::terminal::size() }
    /// Attempt to read the terminal size in characters using only ANSI escape sequences
    ///
    /// It is not guaranteed to work, although more universal than a direct x-term style ANSI window size request "\x1B[18t".
    /// Works best in raw alternate screen mode.
    /// relies on the user to press enter because we cannot read stdout.
    /// WARNING: this is a blocking call
    #[cfg(any(target_family = "wasm", not(feature = "crossterm")))]
    fn size(&mut self) -> io::Result<(u16, u16)> {
        use io::Read;
        // if terms who don't support cursor report at least export COLUMNS and LINES, then we can use that, even if it's not accurate
        if let Ok(s) = std::env::var("COLUMNS").and_then(|cols| std::env::var("LINES").map(|lines| (cols, lines))) {
            if let (Ok(cols), Ok(lines)) = (s.0.parse(), s.1.parse()) {
                return Ok((cols, lines));
            }
        }
        // otherwise, we can try to get the cursor position, but this is not guaranteed to work and user might have to press enter

        eprintln!("Requesting terminal size report, please press enter when a report appears (e.g. \"^[[40;132R\")");
        eprintln!("If no report appears, then you may need to set --width and/or --height with --inline.");
        self.cursor_save()?;
        self.cursor_to(0x7FFF, 0x7FFF)?;
        self.write_all([REPORT_CURSOR_POSITION, RESTORE_CURSOR].concat().as_bytes())?;
        self.flush()?;
        let mut buf = [0; 1];
        let mut s = Vec::<u8>::with_capacity(10);
        loop {
            io::stdin().read_exact(&mut buf)?;
            match buf[0] {
                b'0'..=b'9' | b';' => s.push(buf[0]),
                b'\0' | b'\n' | b'\r' | b'R' => break,
                _ => continue,
            }
        }
        if let Ok(s) = String::from_utf8(s) {
            let mut iter = s.split(';');
            if let (Some(y), Some(x)) = (iter.next(), iter.next()) {
                if let (Ok(x), Ok(y)) = (x.parse::<u16>(), y.parse::<u16>()) {
                    return Ok((x + 1, y + 1));
                }
            }
        }
        error!("Failed to parse terminal size report, defaulting to {}x{}", crate::DEFAULT_COLS, crate::DEFAULT_ROWS);
        Ok((crate::DEFAULT_COLS, crate::DEFAULT_ROWS))
    }
    #[inline]
    fn cursor_hide(&mut self) -> io::Result<()> { self.write_all(HIDE_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_show(&mut self) -> io::Result<()> { self.write_all(SHOW_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_save(&mut self) -> io::Result<()> { self.write_all(SAVE_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_restore(&mut self) -> io::Result<()> { self.write_all(RESTORE_CURSOR.as_bytes()) }
    #[inline]
    fn cursor_report_position(&mut self) -> io::Result<()> { self.write_all(REPORT_CURSOR_POSITION.as_bytes()) }
    #[inline]
    fn cursor_next_line(&mut self) -> io::Result<()> { self.write_all(CURSOR_NEXT_LINE.as_bytes()) }
    #[inline]
    fn cursor_prev_line(&mut self) -> io::Result<()> { self.write_all(CURSOR_PREV_LINE.as_bytes()) }
    #[inline]
    fn cursor_home(&mut self) -> io::Result<()> { self.write_all(CURSOR_HOME.as_bytes()) }
    fn cursor_to(&mut self, x: u16, y: u16) -> io::Result<()> { write!(self, csi!("{};{}H"), y + 1, x + 1) }
    fn cursor_to_col(&mut self, x: u16) -> io::Result<()> { write!(self, csi!("{}G"), x + 1) }
    fn cursor_up(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}A"), n) }
    fn cursor_down(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}B"), n) }
    fn cursor_foward(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}C"), n) }
    fn cursor_backward(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}D"), n) }
    fn cursor_next_lines(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}E"), n) }
    fn cursor_prev_lines(&mut self, n: u16) -> io::Result<()> { write!(self, csi!("{}F"), n) }
    #[inline]
    fn attr_reset(&mut self) -> io::Result<()> { self.write_all(SGR_DEFAULT.as_bytes()) }
}
impl<'a> TerminalImpl for io::StdoutLock<'a> {}
impl TerminalImpl for io::Stdout {}
