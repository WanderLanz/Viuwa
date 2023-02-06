#![doc = include_str!("../README.md")]

#[cfg(feature = "rayon")]
use rayon::prelude::*;
use viuwa_image::*;

mod private {
    pub trait Sealed {}
}
use private::Sealed;
#[macro_use]
mod macros;
pub mod consts;
mod statics;
#[doc(inline)]
pub use statics::*;
mod traits;
#[doc(inline)]
pub use traits::*;
pub mod color;
use color::*;
pub use color::{ColorAttributes, ColorDepth, ColorPresets, ColorSpace, ColorType};
mod pixel;
#[doc(inline)]
pub use pixel::*;
pub mod image;
pub use crate::image::{AnsiImage, DynamicAnsiImage};

/// ```'▄'``` (U+2584) in UTF-8 codepoints. A default `Lower` `Order` character.
pub const LOWER_HALF_BLOCK: Char = Char([0xE2, 0x96, 0x84, 0]); // Char::from_char('▀');
/// ```'▀'``` (U+2580) in UTF-8 codepoints. A default `Upper` `Order` character.
pub const UPPER_HALF_BLOCK: Char = Char([0xE2, 0x96, 0x80, 0]); // Char::from_char('▀');

/// [`PixelRepr`] with `u8` [`Scalar`].
pub trait Bytes: PixelRepr<Scalar = u8> {}
impl<T: PixelRepr<Scalar = u8>> Bytes for T {}

/// A UTF-8 character in raw codepoints. (encoded char)
///
/// This allows us to use a `char` as bytes, which is useful for any transmutations because ```align_of::<char>() != align_of::<[u8; 4]>()```.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Char(pub [u8; 4]);
impl Char {
    /// Get the raw codepoints of this `Char`.
    #[inline(always)]
    pub const fn into_inner(self) -> [u8; 4] { self.0 }
    /// Create a new `Char` from a `char`.<br>
    /// If you know the `char` in advance, instead create a Char` directly with the respective UTF-8 codepoints as `[u8; 4]`.
    ///
    /// HINT: You can use a rust compiler error to tell you the UTF-8 codepoints of a `char`: `const _: &[u8] = b"▀";` will tell you that `char` is `0xE2, 0x96, 0x80, 0`.
    #[inline]
    pub const fn from_char(char: char) -> Self {
        let char = char as u32;
        let mut bytes = [0; 4];
        let len: u8 = if char < 0x80 {
            1
        } else if char < 0x800 {
            2
        } else if char < 0x10000 {
            3
        } else {
            4
        };
        match len {
            1 => {
                bytes[0] = char as u8;
            }
            2 => {
                bytes[0] = (char >> 6 & 0x1F) as u8 | 0xC0;
                bytes[1] = (char & 0x3F) as u8 | 0x80;
            }
            3 => {
                bytes[0] = (char >> 12 & 0x0F) as u8 | 0xE0;
                bytes[1] = (char >> 6 & 0x3F) as u8 | 0x80;
                bytes[2] = (char & 0x3F) as u8 | 0x80;
            }
            4 => {
                bytes[0] = (char >> 18 & 0x07) as u8 | 0xF0;
                bytes[1] = (char >> 12 & 0x3F) as u8 | 0x80;
                bytes[2] = (char >> 6 & 0x3F) as u8 | 0x80;
                bytes[3] = (char & 0x3F) as u8 | 0x80;
            }
            _ => unreachable!(),
        }
        Self(bytes)
    }
}
impl From<Char> for [u8; 4] {
    #[inline(always)]
    fn from(char: Char) -> Self { char.0 }
}
impl From<char> for Char {
    #[inline(always)]
    fn from(char: char) -> Self { Self::from_char(char) }
}
