#![doc = include_str!("../README.md")]

// Maybe make a PR for crossterm to add pure ansi backup when we aren't compiling for UNIX/Windows?
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use viuwa_image::*;

mod private {
    pub trait Sealed {}
}
use private::Sealed;
#[macro_use]
pub mod macros;
pub mod consts;
use consts::*;
mod statics;
#[doc(inline)]
pub use statics::*;
mod traits;
#[doc(inline)]
pub use traits::*;
pub mod color;
use color::*;
pub use color::{ColorAttributes, ColorPresets, ColorType};
mod pixel;
#[doc(inline)]
pub use pixel::*;
mod image;

/// A reasonable default width for the terminal. This is used when the terminal width cannot be determined.
pub const DEFAULT_COLS: u16 = 80;
/// A reasonable default height for the terminal. This is used when the terminal height cannot be determined.
pub const DEFAULT_ROWS: u16 = 24;
// const LOWER_HALF_BLOCK: &str = "\u{2584}";
/// The default visible character to use for an ANSI "pixel" (a character cell)
pub const UPPER_HALF_BLOCK: [u8; 3] = *b"\xe2\x96\x80"; // "\u{2580}";
pub const UPPER_HALF_BLOCK_FULL: [u8; 4] = *b"\xe2\x96\x80\0"; // "\u{2580}";

/// A `u8` or array of `u8`'s
///
/// Any [`PixelRepr`] with a [`Scalar`] of `u8`
pub trait Bytes: PixelRepr<Scalar = u8> {}
impl<R: PixelRepr<Scalar = u8>> Bytes for R {}

/// An array of `u8`'s with len of 4.
/// Used in iterators because const generics are not yet stable
pub type CharBytes = [u8; 4];

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
