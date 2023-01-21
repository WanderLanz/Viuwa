#![doc = include_str!("../README.md")]
#[cfg(feature = "rayon")]
use ::rayon::prelude::*;

// NOTE: prefer to leave as much possible as pub,
// so that users can use specific functionality if necessary.
// Undocumented functionality is better than functionality that is visible but not usable.

mod private {
    /// Sealed trait to prevent external implementations of traits
    pub trait Sealed {}
}
use private::Sealed;
#[doc(hidden)]
pub mod filter;
#[doc(inline)]
pub use crate::filter::FilterType;
use crate::filter::*;
#[doc(hidden)]
pub mod sample;
use crate::sample::*;
mod image;
pub use crate::image::*;
mod pixel;
pub use crate::pixel::*;

/// The maximum size of an image (in bytes) that can be processed safely by this library. (4GB)
///
/// This is an implementation detail for [`viuwa`](https://docs.rs/viuwa/latest/viuwa/).
///
/// If you need to process larger images, you can use unsafe methods or use `from_raw`.
pub const MAX_IMAGE_SIZE: usize = u32::MAX as usize;

/// The type of coefficients (float) used for weights in default sampling
#[doc(hidden)]
pub type Weight = f32; // | f64;
