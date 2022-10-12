// migrate to ansi_colours for truecolor<->256 conversion because our behavior and ui is starting to become stable
// give them some love at https://github.com/mina86/ansi_colours
pub use ::ansi_colours::ansi256_from_grey as grey_to_256;
pub use ::ansi_colours::ansi256_from_rgb as rgb_to_256;
pub fn set_fg24_grey(g: u8) -> String { format!(csi!("38;2;{};{};{}m"), g, g, g) }
pub fn set_bg24_grey(g: u8) -> String { format!(csi!("48;2;{};{};{}m"), g, g, g) }
pub fn set_fg24_color([r, g, b]: [u8; 3]) -> String { format!(csi!("38;2;{};{};{}m"), r, g, b) }
pub fn set_bg24_color([r, g, b]: [u8; 3]) -> String { format!(csi!("48;2;{};{};{}m"), r, g, b) }
pub fn set_fg8(c: u8) -> String { format!(csi!("38;5;{}m"), c) }
pub fn set_bg8(c: u8) -> String { format!(csi!("48;5;{}m"), c) }
