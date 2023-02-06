/// Escapes a string to be printed to the terminal
#[macro_export]
macro_rules! esc {
    ($( $a:expr ),* $(,)?) => { concat!('\x1B', $( $a ),*) };
}
/// Control Sequence Inducer (CSI)
#[macro_export]
macro_rules! csi {
    ($( $a:expr ),* $(,)?) => { $crate::esc!('[', $( $a ),*) };
}
/// Operating System Command (OSC)
#[macro_export]
macro_rules! osc {
    ($( $a:expr ),* $(,)?) => { $crate::esc!(']', $( $a ),*) };
}
/// Device Control String (DCS)
#[macro_export]
macro_rules! dcs {
        ($( $a:expr ),* $(,)?) => { $crate::esc!('P', $( $a ),*) };
}
/// Application Program Command (APC)
#[macro_export]
macro_rules! apm {
        ($( $a:expr ),* $(,)?) => { $crate::esc!('_', $( $a ),*) };
}
/// String Terminator (ST)
#[macro_export]
macro_rules! st {
    ($( $a:expr ),* $(,)?) => { concat!($( $a ),*, $crate::esc!('\\')) };
}
/// Select Graphic Rendition (SGR)
#[macro_export]
macro_rules! sgr {
    ($( $a:expr ),* $(,)?) => { $crate::csi!($($a),*, 'm') };
}

/// use the `fg!` macro to color the foreground of the terminal
///
/// `fg!(color)` will return a string that will set the foreground color of the terminal to `color`
///
/// `fg!(reset)` will return a string that will reset the foreground color of the terminal to the default
/// ### Preset colors
/// - black
/// - red
/// - green
/// - yellow
/// - blue
/// - magenta
/// - cyan
/// - white
///
/// for any preset color, you may capitalize the first letter to use a emphasized (brighter and/or bolder) version on terminals that support it
/// ### 256 color mode
/// use any of the 256 colors by passing a number between 0 and 255 on terminals that support it
/// ### 24-bit color mode
/// use any of the 16 million colors by passing a tuple of 3 numbers (red, green, blue) between 0 and 255 on terminals that support it
#[macro_export]
macro_rules! fg {
    (reset) => {
        $crate::sgr!("39")
    };
    ($col:ident) => {
        $crate::preset_fg!($col)
    };
    ($ansi:literal) => {
        $crate::sgr!("38;5;", stringify!($ansi))
    };
    ($r:literal, $g:literal, $b:literal) => {
        $crate::sgr!("38;2;", stringify!($r), ";", stringify!($g), ";", stringify!($b))
    };
}

/// helper macro for the [`fg`] macro to get preset color sequences' literals
#[macro_export]
macro_rules! preset_fg {
    (black) => {
        $crate::sgr!("30")
    };
    (red) => {
        $crate::sgr!("31")
    };
    (green) => {
        $crate::sgr!("32")
    };
    (yellow) => {
        $crate::sgr!("33")
    };
    (blue) => {
        $crate::sgr!("34")
    };
    (magenta) => {
        $crate::sgr!("35")
    };
    (cyan) => {
        $crate::sgr!("36")
    };
    (white) => {
        $crate::sgr!("37")
    };
    (Black) => {
        $crate::sgr!("90")
    };
    (Red) => {
        $crate::sgr!("91")
    };
    (Green) => {
        $crate::sgr!("92")
    };
    (Yellow) => {
        $crate::sgr!("93")
    };
    (Blue) => {
        $crate::sgr!("94")
    };
    (Magenta) => {
        $crate::sgr!("95")
    };
    (Cyan) => {
        $crate::sgr!("96")
    };
    (White) => {
        $crate::sgr!("97")
    };
}

/// use the `bg!` macro to color the foreground of the terminal
///
/// `bg!(color)` will return a string that will set the background color of the terminal to `color`
///
/// `bg!(reset)` will return a string that will reset the background color of the terminal to the default
/// ### Preset colors
/// - black
/// - red
/// - green
/// - yellow
/// - blue
/// - magenta
/// - cyan
/// - white
///
/// for any preset color, you may capitalize the first letter to use a emphasized (brighter and/or bolder) version on terminals that support it
/// ### 256 color mode
/// use any of the 256 colors by passing a number between 0 and 255 on terminals that support it
/// ### 24-bit color mode
/// use any of the 16 million colors by passing a tuple of 3 numbers (red, green, blue) between 0 and 255 on terminals that support it
#[macro_export]
macro_rules! bg {
    (reset) => {
        $crate::sgr!("49")
    };
    ($col:ident) => {
        $crate::preset_bg!($col)
    };
    ($ansi:literal) => {
        $crate::sgr!("48;5;", stringify!($ansi))
    };
    ($r:literal, $g:literal, $b:literal) => {
        $crate::sgr!("48;2;", stringify!($r), ';', stringify!($g), ';', stringify!($b))
    };
}

/// helper macro for the `bg!` macro to get preset color sequences' literals
#[macro_export]
macro_rules! preset_bg {
    (black) => {
        $crate::sgr!("40")
    };
    (red) => {
        $crate::sgr!("41")
    };
    (green) => {
        $crate::sgr!("42")
    };
    (yellow) => {
        $crate::sgr!("43")
    };
    (blue) => {
        $crate::sgr!("44")
    };
    (magenta) => {
        $crate::sgr!("45")
    };
    (cyan) => {
        $crate::sgr!("46")
    };
    (white) => {
        $crate::sgr!("47")
    };
    (Black) => {
        $crate::sgr!("100")
    };
    (Red) => {
        $crate::sgr!("101")
    };
    (Green) => {
        $crate::sgr!("102")
    };
    (Yellow) => {
        $crate::sgr!("103")
    };
    (Blue) => {
        $crate::sgr!("104")
    };
    (Magenta) => {
        $crate::sgr!("105")
    };
    (Cyan) => {
        $crate::sgr!("106")
    };
    (White) => {
        $crate::sgr!("107")
    };
}

/// Macro for executing a series of fallible write functions on a Terminal, <br>
/// essentially using `and_then` to chain the results of each function call
/// # Example
/// ```ignore
/// use std::io::Write;
/// use std::io::stdout;
/// use viuwa_ansi::{execute, Terminal};
/// let mut term = stdout().lock();
/// execute!(
///     term,
///     clear_screen(),
///     write_all(b"Hello, "),
///     write_all(b"world!"),
///     cursor_foward(7),
///     write_all(b"beautiful "),
///     write_all(b"world!"),
///     cursor_home(),
/// ).expect("Failed to write to terminal");
/// ```
#[macro_export]
macro_rules! execute {
    ($i:expr, $f:ident($($a:expr),*)$(, $fr:ident($($ar:expr),*))+) => {
        match $i.$f($($a),*) {
            Ok(_) => execute!($i, $($fr($($ar),*)),+),
            Err(e) => Err(e),
        }
    };
    ($i:expr, $f:ident($($a:expr),*)) => {
        $i.$f($($a),*)
    };
}
