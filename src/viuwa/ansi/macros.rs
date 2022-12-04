#[macro_export]
macro_rules! esc {
    ($( $l:expr ),*) => { concat!('\x1B', $( $l ),*) };
}
#[macro_export]
macro_rules! csi {
    ($( $l:expr ),*) => { concat!(esc!('['), $( $l ),*) };
}
#[macro_export]
macro_rules! osc {
    ($( $l:expr ),*) => { concat!(esc!(']'), $( $l ),*) };
}
// macro_rules! dcs {
//         ($( $l:expr ),*) => { concat!(esc!('P'), $( $l ),*) };
// }
// macro_rules! apm {
//         ($( $l:expr ),*) => { concat!(esc!('_'), $( $l ),*) };
// }
#[macro_export]
macro_rules! st {
    ($( $l:expr ),*) => { concat!($( $l ),*, esc!('\\')) };
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
        csi!("39m")
    };
    ($col:ident) => {
        fg_preset!($col)
    };
    ($ansi:literal) => {
        concat!(csi!("38;5;", stringify!($ansi), "m"))
    };
    ($r:literal, $g:literal, $b:literal) => {
        concat!(csi!("38;2;", stringify!($r), ";", stringify!($g), ";", stringify!($b), "m"))
    };
}

/// helper macro for the `fg!` macro to get preset color sequences' literals
#[macro_export]
macro_rules! fg_preset {
    (black) => {
        csi!("30m")
    };
    (red) => {
        csi!("31m")
    };
    (green) => {
        csi!("32m")
    };
    (yellow) => {
        csi!("33m")
    };
    (blue) => {
        csi!("34m")
    };
    (magenta) => {
        csi!("35m")
    };
    (cyan) => {
        csi!("36m")
    };
    (white) => {
        csi!("37m")
    };
    (Black) => {
        csi!("90m")
    };
    (Red) => {
        csi!("91m")
    };
    (Green) => {
        csi!("92m")
    };
    (Yellow) => {
        csi!("93m")
    };
    (Blue) => {
        csi!("94m")
    };
    (Magenta) => {
        csi!("95m")
    };
    (Cyan) => {
        csi!("96m")
    };
    (White) => {
        csi!("97m")
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
        csi!("49m")
    };
    ($col:ident) => {
        bg_preset!($col)
    };
    ($ansi:literal) => {
        concat!(csi!("48;5;", stringify!($ansi), "m"))
    };
    ($r:literal, $g:literal, $b:literal) => {
        concat!(csi!("48;2;", stringify!($r), ";", stringify!($g), ";", stringify!($b), "m"))
    };
}

/// helper macro for the `bg!` macro to get preset color sequences' literals
#[macro_export]
macro_rules! bg_preset {
    (black) => {
        csi!("40m")
    };
    (red) => {
        csi!("41m")
    };
    (green) => {
        csi!("42m")
    };
    (yellow) => {
        csi!("43m")
    };
    (blue) => {
        csi!("44m")
    };
    (magenta) => {
        csi!("45m")
    };
    (cyan) => {
        csi!("46m")
    };
    (white) => {
        csi!("47m")
    };
    (Black) => {
        csi!("100m")
    };
    (Red) => {
        csi!("101m")
    };
    (Green) => {
        csi!("102m")
    };
    (Yellow) => {
        csi!("103m")
    };
    (Blue) => {
        csi!("104m")
    };
    (Magenta) => {
        csi!("105m")
    };
    (Cyan) => {
        csi!("106m")
    };
    (White) => {
        csi!("107m")
    };
}
