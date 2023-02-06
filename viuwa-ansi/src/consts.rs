//! The most commonly recognized ANSI sequences by most terminals

// xterm reports
// avoid as much as possible
// /// -> `CSI  8 ;  height ;  width t`.
// const REPORT_WINDOW_CHAR_SIZE: &str = csi!("18t");
// /// -> `CSI  9 ;  height ;  width t`.
// const REPORT_SCREEN_CHAR_SIZE: &str = csi!("19t");
// /// -> `OSC  L  label ST`
// const REPORT_WINDOW_ICON_LABEL: &str = csi!("20t");
// /// -> `OSC  l  label ST`
// const REPORT_WINDOW_TITLE: &str = csi!("21t");

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
/// Reset the terminal, incompatible with Windows
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
// pub const START_SIXEL: &str = dcs!("Pq");

/// Scroll up 1 line
pub const SCROLL_UP: &str = csi!("1S");
/// Scroll down 1 line
pub const SCROLL_DOWN: &str = csi!("1T");

/// Hide the cursor
pub const HIDE_CURSOR: &str = csi!("?25l");
/// Show the cursor
pub const SHOW_CURSOR: &str = csi!("?25h");
/// Save the cursor position and attributes
pub const SAVE_CURSOR: &str = esc!("7");
/// Restore the cursor position and attributes
pub const RESTORE_CURSOR: &str = esc!("8");
/// Report the cursor position in the format `ESC [ <row> ; <col> R` (sent to stdin)
pub const REPORT_CURSOR_POSITION: &str = csi!("6n");

/// Move the cursor to the top left corner (can be overwritten by the terminal)
pub const CURSOR_HOME: &str = csi!("H");
/// Move the cursor to the bottom right corner (can be overwritten by the terminal)
pub const CURSOR_END: &str = csi!("F");
/// Move the cursor 1 cell up
pub const CURSOR_UP: &str = csi!("A");
/// Move the cursor 1 cell down
pub const CURSOR_DOWN: &str = csi!("B");
/// Move the cursor 1 position right (forwards)
pub const CURSOR_RIGHT: &str = csi!("C");
/// Move the cursor 1 position left (backwards)
pub const CURSOR_LEFT: &str = csi!("D");

/// Move the cursor to the start of the next line
pub const CURSOR_NEXT_LINE: &str = csi!("1E");
/// Move the cursor to the start of the previous line
pub const CURSOR_PREV_LINE: &str = csi!("1F");

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
/// Reverse FG and BG
pub const SGR_REVERSE: &str = sgr!("7");
/// Unset reverse
pub const SGR_NO_REVERSE: &str = sgr!("27");

pub const FG_BLACK: &str = sgr!("30");
pub const FG_RED: &str = sgr!("31");
pub const FG_GREEN: &str = sgr!("32");
pub const FG_YELLOW: &str = sgr!("33");
pub const FG_BLUE: &str = sgr!("34");
pub const FG_MAGENTA: &str = sgr!("35");
pub const FG_CYAN: &str = sgr!("36");
pub const FG_WHITE: &str = sgr!("37");
pub const FG_DEFAULT: &str = sgr!("39");

pub const BG_BLACK: &str = sgr!("40");
pub const BG_RED: &str = sgr!("41");
pub const BG_GREEN: &str = sgr!("42");
pub const BG_YELLOW: &str = sgr!("43");
pub const BG_BLUE: &str = sgr!("44");
pub const BG_MAGENTA: &str = sgr!("45");
pub const BG_CYAN: &str = sgr!("46");
pub const BG_WHITE: &str = sgr!("47");
pub const BG_DEFAULT: &str = sgr!("49");

pub const FG_BRIGHT_BLACK: &str = sgr!("90");
pub const FG_BRIGHT_RED: &str = sgr!("91");
pub const FG_BRIGHT_GREEN: &str = sgr!("92");
pub const FG_BRIGHT_YELLOW: &str = sgr!("93");
pub const FG_BRIGHT_BLUE: &str = sgr!("94");
pub const FG_BRIGHT_MAGENTA: &str = sgr!("95");
pub const FG_BRIGHT_CYAN: &str = sgr!("96");
pub const FG_BRIGHT_WHITE: &str = sgr!("97");

pub const BG_BRIGHT_BLACK: &str = sgr!("100");
pub const BG_BRIGHT_RED: &str = sgr!("101");
pub const BG_BRIGHT_GREEN: &str = sgr!("102");
pub const BG_BRIGHT_YELLOW: &str = sgr!("103");
pub const BG_BRIGHT_BLUE: &str = sgr!("104");
pub const BG_BRIGHT_MAGENTA: &str = sgr!("105");
pub const BG_BRIGHT_CYAN: &str = sgr!("106");
pub const BG_BRIGHT_WHITE: &str = sgr!("107");
