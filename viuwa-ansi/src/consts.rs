/// Clear the terminal buffer
pub const CLEAR_BUFFER: &str = csi!("3J");
/// Clear the terminal screen
pub const CLEAR_SCREEN: &str = csi!("2J");
/// Clear the terminal screen from cursor to the end
pub const CLEAR_SCREEN_TO_END: &str = csi!("0J");
/// Clear the terminal screen from cursor to the start
pub const CLEAR_SCREEN_TO_START: &str = csi!("1J");
/// Clear the line the cursor is on
pub const CLEAR_LINE: &str = csi!("2K");
/// Clear the line the cursor is on from cursor to the end
pub const CLEAR_LINE_TO_END: &str = csi!("0K");
/// Clear the line the cursor is on from cursor to the start
pub const CLEAR_LINE_TO_START: &str = csi!("1K");
/// Reset the terminal
pub const RESET: &str = esc!("c");
/// Reset the terminal
pub const SOFT_RESET: &str = esc!("!p");

/// Enter alternate screen buffer mode
pub const ENTER_ALT_SCREEN: &str = csi!("?1049h");
/// Exit alternate screen buffer mode
pub const EXIT_ALT_SCREEN: &str = csi!("?1049l");
/// Enable line wrap
pub const ENABLE_LINE_WRAP: &str = csi!("?7h");
/// Disable line wrap
pub const DISABLE_LINE_WRAP: &str = csi!("?7l");
/// use crossterm instead when possible for windows compatibility
/// Enables raw mode: no terminal input processing
pub const ENABLE_RAW_MODE: &str = csi!("?1h");
/// use crossterm instead when possible for windows compatibility
/// Disables raw mode: terminal input processing
pub const DISABLE_RAW_MODE: &str = csi!("?1l");
// pub const START_SIXEL: &str = dcs!("Pq");

/// Hide the cursor
pub const HIDE_CURSOR: &str = csi!("?25l");
/// Show the cursor
pub const SHOW_CURSOR: &str = csi!("?25h");
/// Save the cursor position and attributes
pub const SAVE_CURSOR: &str = esc!("7");
/// Restore the cursor position and attributes
pub const RESTORE_CURSOR: &str = esc!("8");
/// Report the cursor position in the format `ESC [ <row> ; <col> R` (sent to stdout)
pub const REPORT_CURSOR_POSITION: &str = csi!("6n");

/// Move the cursor to the top left corner
pub const CURSOR_HOME: &str = csi!("H");
/// Move the cursor to the bottom right corner
pub const CURSOR_END: &str = csi!("F");
/// Move the cursor 1 position up (use Cursor::move_up for more control)
pub const CURSOR_UP: &str = csi!("A");
/// Move the cursor 1 position down (use Cursor::move_down for more control)
pub const CURSOR_DOWN: &str = csi!("B");
/// Move the cursor 1 position right (use Cursor::move_right for more control)
pub const CURSOR_RIGHT: &str = csi!("C");
/// Move the cursor 1 position left (use Cursor::move_left for more control)
pub const CURSOR_LEFT: &str = csi!("D");

/// Move the cursor to the start of the next line
pub const CURSOR_NEXT_LINE: &str = csi!("1E");
/// Move the cursor to the start of the previous line
pub const CURSOR_PREV_LINE: &str = csi!("1F");

/// Scroll up 1 line
pub const SCROLL_UP: &str = csi!("1S");
/// Scroll down 1 line
pub const SCROLL_DOWN: &str = csi!("1T");
/// Reset all attributes to default
pub const SGR_DEFAULT: &str = sgr!("0");
/// Set bold
pub const SGR_BOLD: &str = sgr!("1");
/// Unset bold
pub const SGR_NO_BOLD: &str = sgr!("22");
/// Set underline
pub const SGR_UNDERLINE: &str = sgr!("4");
/// Unset underline
pub const SGR_NO_UNDERLINE: &str = sgr!("24");
/// Reverse foreground and background
pub const SGR_REVERSE: &str = sgr!("7");
/// Unset reverse
pub const SGR_NO_REVERSE: &str = sgr!("27");

pub const FOREGROUND_BLACK: &str = sgr!("30");
pub const FOREGROUND_RED: &str = sgr!("31");
pub const FOREGROUND_GREEN: &str = sgr!("32");
pub const FOREGROUND_YELLOW: &str = sgr!("33");
pub const FOREGROUND_BLUE: &str = sgr!("34");
pub const FOREGROUND_MAGENTA: &str = sgr!("35");
pub const FOREGROUND_CYAN: &str = sgr!("36");
pub const FOREGROUND_WHITE: &str = sgr!("37");
pub const FOREGROUND_DEFAULT: &str = sgr!("39");

pub const BACKGROUND_BLACK: &str = sgr!("40");
pub const BACKGROUND_RED: &str = sgr!("41");
pub const BACKGROUND_GREEN: &str = sgr!("42");
pub const BACKGROUND_YELLOW: &str = sgr!("43");
pub const BACKGROUND_BLUE: &str = sgr!("44");
pub const BACKGROUND_MAGENTA: &str = sgr!("45");
pub const BACKGROUND_CYAN: &str = sgr!("46");
pub const BACKGROUND_WHITE: &str = sgr!("47");
pub const BACKGROUND_DEFAULT: &str = sgr!("49");

pub const FOREGROUND_BRIGHT_BLACK: &str = sgr!("90");
pub const FOREGROUND_BRIGHT_RED: &str = sgr!("91");
pub const FOREGROUND_BRIGHT_GREEN: &str = sgr!("92");
pub const FOREGROUND_BRIGHT_YELLOW: &str = sgr!("93");
pub const FOREGROUND_BRIGHT_BLUE: &str = sgr!("94");
pub const FOREGROUND_BRIGHT_MAGENTA: &str = sgr!("95");
pub const FOREGROUND_BRIGHT_CYAN: &str = sgr!("96");
pub const FOREGROUND_BRIGHT_WHITE: &str = sgr!("97");

pub const BACKGROUND_BRIGHT_BLACK: &str = sgr!("100");
pub const BACKGROUND_BRIGHT_RED: &str = sgr!("101");
pub const BACKGROUND_BRIGHT_GREEN: &str = sgr!("102");
pub const BACKGROUND_BRIGHT_YELLOW: &str = sgr!("103");
pub const BACKGROUND_BRIGHT_BLUE: &str = sgr!("104");
pub const BACKGROUND_BRIGHT_MAGENTA: &str = sgr!("105");
pub const BACKGROUND_BRIGHT_CYAN: &str = sgr!("106");
pub const BACKGROUND_BRIGHT_WHITE: &str = sgr!("107");
