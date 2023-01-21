use ::anyhow::{anyhow, Result};
#[cfg(feature = "clap")]
use ::clap::ValueEnum;
#[cfg(feature = "fir")]
use ::fast_image_resize as fir;
use ::image::{ImageBuffer, Luma, Rgb};
#[cfg(feature = "rayon")]
use ::rayon::prelude::*;
use ::tracing::{debug, error, info, instrument, span, warn, Level};
use ansi::color::RawPixel;
use num_traits::{NumAssignRef, NumCast, ToPrimitive};

#[macro_use]
pub mod ansi;
pub mod resizer;
mod private {
    pub trait Sealed {}
}
use ::ndarray::prelude::*;
pub use ansi::AnsiImage;
use private::Sealed;
pub use resizer::{FilterType, Resizer};
mod image;
pub use crate::image::*;

/// Explicit uninit macro to avoid reliance on optimizer
#[macro_export]
macro_rules! uninit {
    () => {
        #[allow(invalid_value)]
        unsafe {
            ::core::mem::MaybeUninit::uninit().assume_init()
        }
    };
    ($t:ty) => {
        #[allow(invalid_value)]
        unsafe {
            ::core::mem::MaybeUninit::<$t>::uninit().assume_init()
        }
    };
}

/// A reasonable default width for the terminal. This is used when the terminal width cannot be determined.
#[cfg(any(target_family = "wasm", not(feature = "crossterm")))]
const DEFAULT_COLS: u16 = 80;
/// A reasonable default height for the terminal. This is used when the terminal height cannot be determined.
#[cfg(any(target_family = "wasm", not(feature = "crossterm")))]
const DEFAULT_ROWS: u16 = 24;
// const LOWER_HALF_BLOCK: &str = "\u{2584}";
const UPPER_HALF_BLOCK: &str = "\u{2580}";

/// The type of coefficients (e.g. f32, f64) to use for weights in default sampling
pub type Weight = f32; // | f64;

/// The raw scalar data type of an image (e.g. u8, u16, f32, etc.)
#[cfg(not(feature = "image"))]
pub trait Scalar: 'static + Sealed + Clone + Copy + Send + Sync + Sized + NumAssignRef + NumCast + ToPrimitive {
    #[inline(always)]
    fn as_<T: Scalar>(self) -> T { self as T }
}
/// The raw scalar data type of an image (e.g. u8, u16, f32, etc.)
#[cfg(feature = "image")]
pub trait Scalar:
    'static + Sealed + ::image::Primitive + Clone + Copy + Send + Sync + Sized + NumAssignRef + NumCast + ToPrimitive
{
    #[inline(always)]
    fn as_<T: Scalar>(self) -> T { self as T }
}
macro_rules! impl_scalar {
    ($($t:ty),*) => {
        $(
            impl Sealed for $t {}
            impl Scalar for $t {}
        )*
    }
}
impl_scalar!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, isize, usize);

/// Scalars compatible with all possible crates: `u8`, `u16`, `i32`, and `f32`
#[cfg(not(feature = "fir"))]
pub trait CompatibleScalar: Scalar {}
#[cfg(not(feature = "fir"))]
impl CompatibleScalar for u8 {}
#[cfg(not(feature = "fir"))]
impl CompatibleScalar for u16 {}
#[cfg(not(feature = "fir"))]
impl CompatibleScalar for i32 {}
#[cfg(not(feature = "fir"))]
impl CompatibleScalar for f32 {}
/// Scalars compatible with all possible crates: `u8`, `u16`, `i32`, and `f32`
#[cfg(feature = "fir")]
pub trait CompatibleScalar: Scalar + fir::pixels::PixelComponent {}
#[cfg(feature = "fir")]
impl<T: Scalar + fir::pixels::PixelComponent> CompatibleScalar for T {}

/// Representations of a pixel as an array of scalars
pub trait PixelRepr:
    'static
    + Sealed
    + Clone
    + Copy
    + Send
    + Sync
    + Sized
    + IntoIterator<Item = <Self as PixelRepr>::Scalar>
    + AsRef<[<Self as PixelRepr>::Scalar]>
    + AsMut<[<Self as PixelRepr>::Scalar]>
    + ::core::ops::Index<usize>
    + ::core::ops::IndexMut<usize>
{
    /// The scalar type of the pixel
    type Scalar: Scalar;
    /// The number of channels in the pixel (e.g. 3 for RGB or 1 for grayscale)
    ///
    /// *Workaround for const generics not being available in trait bounds
    const CHANNELS: usize;
}
macro_rules! impl_pixel_repr {
    ($($n:literal),+) => {
        $(
            impl<T: Scalar> Sealed for [T; $n] {}
            impl<T: Scalar> PixelRepr for [T; $n] {
                type Scalar = T;
                const CHANNELS: usize = $n;
            }
        )+
    };
}
// REVIEW: Should we support more than 5 channels or remove restriction?
impl_pixel_repr!(1, 2, 3, 4, 5);

/// Representations of a pixel as an array of scalars, compatible with all possible crates
pub trait CompatiblePixelRepr: PixelRepr
where
    Self::Scalar: CompatibleScalar,
{
}
impl<R: PixelRepr> CompatiblePixelRepr for R where R::Scalar: CompatibleScalar {}

#[cfg(feature = "clap")]
#[derive(clap::ValueEnum, Debug, Clone, Copy, Default)]
pub enum ColorType {
    #[default]
    #[value(name = "truecolor")]
    Color,
    #[value(name = "256")]
    Color256,
    #[value(name = "gray")]
    Gray,
    #[value(name = "256gray")]
    Gray256,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ColorType {
    #[default]
    Color,
    Color256,
    Gray,
    Gray256,
}

impl ColorType {
    #[inline]
    pub fn cycle(&self) -> ColorType {
        match self {
            ColorType::Color => ColorType::Color256,
            ColorType::Color256 => ColorType::Gray,
            ColorType::Gray => ColorType::Gray256,
            ColorType::Gray256 => ColorType::Color,
        }
    }
    #[inline]
    pub fn is_color(&self) -> bool {
        match self {
            ColorType::Color | ColorType::Color256 => true,
            ColorType::Gray | ColorType::Gray256 => false,
        }
    }
    #[inline]
    pub fn is_24bit(&self) -> bool {
        match self {
            ColorType::Color | ColorType::Gray => true,
            ColorType::Color256 | ColorType::Gray256 => false,
        }
    }
}

/// Wrapper around possibly user-controlled color attributes
#[derive(Debug, Clone, Copy)]
pub struct ColorAttributes {
    /// luma correct as a color distance threshold
    pub luma_correct: u32,
}

impl ColorAttributes {
    /// luma correct is 0..=100, 100 is the highest luma correct
    // for n and f(luma_correct) = ((100 - luma_correct)^n / 100^(n-1)), as n increases, the luma correct becomes less aggressive
    // distance threshold = (MAX_COLOR_DISTANCE / 100) * ((100 - luma_correct)^3 / 100^2)
    pub fn new(luma_correct: u32) -> Self {
        Self { luma_correct: (((100 - luma_correct).pow(3) / 10000) as f32 * ansi::color::MAP_0_100_DIST) as u32 }
    }
}
