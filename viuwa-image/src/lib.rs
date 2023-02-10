#![doc = include_str!("../README.md")]
#[cfg(feature = "rayon")]
use ::rayon::{
    iter::{IndexedParallelIterator, ParallelIterator},
    prelude::*,
    slice::{ChunksExact as ParChunksExact, ChunksExactMut as ParChunksExactMut},
};

// NOTE: prefer to leave as much possible as pub,
// so that users can use specific functionality if necessary.
// Undocumented functionality is better than functionality that is visible but not usable.

#[macro_use]
mod macros;

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

/// The maximum size of an image (in bytes) that is processed by this library.
///
/// This is an implementation detail for [`viuwa`](https://docs.rs/viuwa/latest/viuwa/).
/// sorry for the inconvenience, maybe it will be removed in the future, even if 4GB is likely to be enough for everyone.
pub const MAX_IMAGE_SIZE: usize = u32::MAX as usize;

/// The type of coefficients (float) used for weights in default sampling, eventually removed once it's decided
#[doc(hidden)]
pub type Weight = f32; // | f64;
