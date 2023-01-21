//! Scalar, Pixel Representation, and Pixel types and traits.
//!
//! Use [`CompatScalar`], [`CompatPixelRepr`], and [`CompatPixel`] to make your code compatible with all possible crates and features.
//! ```
//! use fast_image_resize::pixel::PixelComponent;
//! use viuwa_image::pixel::{CompatScalar, CompatPixelRepr};
//! let count = <<[u8; 4] as CompatPixelRepr>::Scalar as PixelComponent>::count_of_values();
//! ```
//! ## Compatible Pixel Representations and their `fast_image_resize` equivalents
//!```ignore
//! u8 => U8,
//! u16 => U16,
//! i32 => I32,
//! f32 => F32,
//! [u8; 1] => U8,
//! [u8; 2] => U8x2,
//! [u8; 3] => U8x3,
//! [u8; 4] => U8x4,
//! [u16; 1] => U16,
//! [u16; 2] => U16x2,
//! [u16; 3] => U16x3,
//! [u16; 4] => U16x4,
//! [i32; 1] => I32,
//! [f32; 1] => F32,
//! ```

// NOTE: More complicated than necessary for compatibility (the `fast_image_resize` crate uses a private trait for convolution),
// this is functionally actually very simple, don't think about it too much.

// Why not implement SIMD ourselves? Because it's a lot of work, Kirill (@Cykooz) and the team have done a fine job, and we don't need to.

// Why not just use nightly? Because we don't want to force nightly on users.

// Solution: Just deal with compatibility.

use super::*;

macro_rules! def_Scalar {
    ($(+ $t:path),*) => {
        /// The raw scalar data type of an image (e.g. `u8`, `u16`, `f32`, etc.)
        /// ```
        /// fn test() {
        ///     use viuwa_image::pixel::Scalar;
        ///     let _: u8 = 0u8.as_();
        ///     let _: u16 = 0u8.as_();
        ///     let _: i32 = 0u8.as_();
        /// }
        /// ```
        pub trait Scalar:
            'static
            + Sealed
            + Clone
            + Copy
            + Send
            + Sync
            + Sized
            + ::bytemuck::Pod
            + ::num_traits::Num
            + ::num_traits::NumAssignRef
            + ::num_traits::NumCast
            + ::num_traits::ToPrimitive
            $(+ $t)*
        {
            /// The `0` value of this scalar type
            const ZERO: Self;
            /// The `1` value of this scalar type
            const ONE: Self;
            /// The maximum value of this scalar type
            const MAX: Self;
            /// The minimum value of this scalar type
            const MIN: Self;
            /// self as Weight (float)
            fn weight(self) -> Weight;
            /// Weight (float) as self
            fn scalar(weight: Weight) -> Self;
        }
    };
}

#[cfg(not(feature = "image"))]
mod compat_image {
    use super::*;
    def_Scalar!();
}
#[cfg(feature = "image")]
mod compat_image {
    use super::*;
    def_Scalar!(+ ::image::Primitive);
    impl<T: Scalar> Pixel for ::image::Rgb<T> {
        type Repr = [T; 3];
    }
    impl<T: Scalar> Pixel for ::image::Luma<T> {
        type Repr = T;
    }
    impl<T: Scalar> Pixel for ::image::Rgba<T> {
        type Repr = [T; 4];
    }
    impl<T: Scalar> Pixel for ::image::LumaA<T> {
        type Repr = [T; 2];
    }
}
pub use self::compat_image::*;

#[cfg(not(feature = "fir"))]
mod compat_fir {
    use super::*;
    /// Scalars compatible with all possible crates: `u8`, `u16`, `i32`, and `f32`
    pub trait CompatScalar: Scalar {}
    impl CompatScalar for u8 {}
    impl CompatScalar for u16 {}
    impl CompatScalar for i32 {}
    impl CompatScalar for f32 {}

    /// Representations of a pixel as an array of scalars, compatible with all possible crates
    pub trait CompatPixelRepr: PixelRepr
    where
        Self::Scalar: CompatScalar,
    {
    }
    macro_rules! impl_CompatPixelRepr {
        ($([$($T:ty),+; $N:literal]),+) => {
            $($(impl CompatPixelRepr for [$T; $N] {})+)+
        };
        ($($T:ty),+) => {
            $(impl CompatPixelRepr for $T {})+
        }
    }
    impl_CompatPixelRepr!(u8, u16, i32, f32);
    impl_CompatPixelRepr!([u8,u16,i32,f32; 1], [u8,u16; 2], [u8,u16; 3], [u8,u16; 4]);

    /// Pixel types that can be used with this crate and `viuwa-ansi`, compatible with all features.
    pub trait CompatPixel: Pixel
    where
        <Self::Repr as PixelRepr>::Scalar: CompatScalar,
        Self::Repr: CompatPixelRepr,
    {
    }
    impl<P: Pixel> CompatPixel for P
    where
        <P::Repr as PixelRepr>::Scalar: CompatScalar,
        P::Repr: CompatPixelRepr,
    {
    }
}
#[cfg(feature = "fir")]
mod compat_fir {
    use ::fast_image_resize::{
        pixels::PixelComponent, DynamicImageView, DynamicImageViewMut, ImageView as FirImageView,
        ImageViewMut as FirImageViewMut,
    };

    use super::*;

    /// Scalars compatible with all possible crates: `u8`, `u16`, `i32`, and `f32`
    pub trait CompatScalar: Scalar + PixelComponent {}
    impl<T: Scalar + PixelComponent> CompatScalar for T {}

    #[inline(always)]
    fn fir_dimensions((w, h): (usize, usize)) -> (::core::num::NonZeroU32, ::core::num::NonZeroU32) {
        unsafe { (::core::num::NonZeroU32::new_unchecked(w as u32), ::core::num::NonZeroU32::new_unchecked(h as u32)) }
    }

    /// Representations of a pixel as an array of scalars, compatible with all possible crates
    pub trait CompatPixelRepr: PixelRepr
    where
        Self::Scalar: CompatScalar,
    {
        /// Internal function for converting an image to a `DynamicImageView` for use with `fast_image_resize`.
        fn fir_view<'a, P: Pixel<Repr = Self>>(image: ImageView<'a, P>) -> DynamicImageView<'a>;
        /// Internal function for converting an image to a `DynamicImageView` for use with `fast_image_resize`.
        fn fir_view_mut<'a, P: Pixel<Repr = Self>>(image: ImageViewMut<'a, P>) -> DynamicImageViewMut<'a>;
    }
    /// Simply because `fast_image_resize` doesn't expose the `Convolution` trait
    macro_rules! impl_CompatPixelRepr {
        ($([$T:ty; $N:literal] => $P:ident),+ $(,)?) => {
            $(
                impl CompatPixelRepr for [$T; $N] {
                    #[inline(always)]
                    fn fir_view<'a, P: Pixel<Repr = Self>>(image: ImageView<'a, P>) -> DynamicImageView<'a> {
                        let (w,h) = fir_dimensions(image.dimensions());
                        let len = image.data().len() * ::core::mem::size_of::<Self::Scalar>();
                        let Ok(view) = FirImageView::<::fast_image_resize::pixels::$P>::from_buffer(w, h, unsafe { &*::core::ptr::slice_from_raw_parts(image.data() as *const _ as *const _, len) }) else { panic!("Tried to create a DynamicImageView with a zero dimension"); };
                        DynamicImageView::from(view)
                    }
                    #[inline(always)]
                    fn fir_view_mut<'a, P: Pixel<Repr = Self>>(mut image: ImageViewMut<'a, P>) -> DynamicImageViewMut<'a> {
                        let (w,h) = fir_dimensions(image.dimensions());
                        let len = image.data().len() * ::core::mem::size_of::<Self::Scalar>();
                        let Ok(view) = FirImageViewMut::<::fast_image_resize::pixels::$P>::from_buffer(w, h, unsafe { &mut *::core::ptr::slice_from_raw_parts_mut(image.data_mut() as *mut _ as *mut _, len) }) else { panic!("Tried to create a DynamicImageViewMut with a zero dimension"); };
                        DynamicImageViewMut::from(view)
                    }
                }
            )+
        };
        ($($T:ty => $P:ident),+ $(,)?) => {
            $(
                impl CompatPixelRepr for $T {
                    #[inline(always)]
                    fn fir_view<'a, P: Pixel<Repr = Self>>(image: ImageView<'a, P>) -> DynamicImageView<'a> {
                        let (w,h) = fir_dimensions(image.dimensions());
                        let len = image.data().len() * ::core::mem::size_of::<Self>();
                        let Ok(view) = FirImageView::<::fast_image_resize::pixels::$P>::from_buffer(w, h, unsafe { &*::core::ptr::slice_from_raw_parts(image.data() as *const _ as *const _, len) }) else { panic!("Tried to create a DynamicImageView with a zero dimension"); };
                        DynamicImageView::from(view)
                    }
                    #[inline(always)]
                    fn fir_view_mut<'a, P: Pixel<Repr = Self>>(mut image: ImageViewMut<'a, P>) -> DynamicImageViewMut<'a> {
                        let (w,h) = fir_dimensions(image.dimensions());
                        let len = image.data().len() * ::core::mem::size_of::<Self>();
                        let Ok(view) = FirImageViewMut::<::fast_image_resize::pixels::$P>::from_buffer(w, h, unsafe { &mut *::core::ptr::slice_from_raw_parts_mut(image.data_mut() as *mut _ as *mut _, len) }) else { panic!("Tried to create a DynamicImageViewMut with a zero dimension"); };
                        DynamicImageViewMut::from(view)
                    }
                }
            )+
        };
    }
    impl_CompatPixelRepr!(u8 => U8, u16 => U16, i32 => I32, f32 => F32);
    impl_CompatPixelRepr! {
        [u8; 1] => U8,
        [u8; 2] => U8x2,
        [u8; 3] => U8x3,
        [u8; 4] => U8x4,
        [u16; 1] => U16,
        [u16; 2] => U16x2,
        [u16; 3] => U16x3,
        [u16; 4] => U16x4,
        [i32; 1] => I32,
        [f32; 1] => F32,
    }

    /// Pixel types that can be used with this crate and `viuwa-ansi`, compatible with all features.
    pub trait CompatPixel: Pixel
    where
        <Self::Repr as PixelRepr>::Scalar: CompatScalar,
        Self::Repr: CompatPixelRepr,
    {
        /// Internal function for converting an image to a `DynamicImageView` for use with `fast_image_resize`.
        #[inline(always)]
        fn fir_view<'a>(image: ImageView<'a, Self>) -> DynamicImageView<'a> { Self::Repr::fir_view(image) }
        /// Internal function for converting an image to a `DynamicImageView` for use with `fast_image_resize`.
        #[inline(always)]
        fn fir_view_mut<'a>(image: ImageViewMut<'a, Self>) -> DynamicImageViewMut<'a> { Self::Repr::fir_view_mut(image) }
    }
    impl<P: Pixel> CompatPixel for P
    where
        <P::Repr as PixelRepr>::Scalar: CompatScalar,
        P::Repr: CompatPixelRepr,
    {
    }
}
pub use self::compat_fir::*;

macro_rules! impl_Scalar_int {
    ($($t:ty),*) => {
        $(
            impl Sealed for $t {}
            impl Scalar for $t {
                const ZERO: Self = 0;
                const ONE: Self = 1;
                const MAX: Self = <$t>::MAX;
                const MIN: Self = <$t>::MIN;
                #[inline(always)]
                fn weight(self) -> Weight { self as Weight }
                #[inline(always)]
                fn scalar(weight: Weight) -> Self { weight as Self }
            }
        )*
    }
}
macro_rules! impl_Scalar_float {
    ($($t:ty),*) => {
        $(
            impl Sealed for $t {}
            impl Scalar for $t {
                const ZERO: Self = 0.;
                const ONE: Self = 1.;
                const MAX: Self = <$t>::MAX;
                const MIN: Self = <$t>::MIN;
                #[inline(always)]
                fn weight(self) -> Weight { self as Weight }
                #[inline(always)]
                fn scalar(weight: Weight) -> Self { weight as Self }
            }
        )*
    }
}
impl_Scalar_int!(u8, u16, u32, u64, i8, i16, i32, i64, isize, usize);
impl_Scalar_float!(f32, f64);

/// Representations of a pixel as an array of scalars
pub trait PixelRepr: 'static + Sealed + Clone + Copy + Send + Sync + Sized + ::bytemuck::Pod {
    /// The scalar type of the pixel
    type Scalar: Scalar;
    /// The number of channels in the pixel (e.g. 3 for RGB or 1 for grayscale)
    ///
    /// *Workaround for const generics not being available in trait bounds
    const CHANNELS: usize;
    /// The repr with each scalar as 0
    const ZERO: Self;
    /// The repr with each scalar as 1
    const ONE: Self;
    /// Appropriately sized weights for the pixel, convenience to avoid unsafe code
    type Weights: PixelRepr<Scalar = Weight>;
    /// Returns a slice of the pixel's scalars
    #[inline(always)]
    fn as_slice(&self) -> &[<Self as PixelRepr>::Scalar] {
        unsafe { &*::core::ptr::slice_from_raw_parts(self as *const Self as *const _, Self::CHANNELS) }
    }
    /// Returns a mutable slice of the pixel's scalars
    #[inline(always)]
    fn as_slice_mut(&mut self) -> &mut [<Self as PixelRepr>::Scalar] {
        unsafe { &mut *::core::ptr::slice_from_raw_parts_mut(self as *mut Self as *mut _, Self::CHANNELS) }
    }
}
impl<T: Scalar, const N: usize> Sealed for [T; N] {}
impl<T: Scalar, const N: usize> PixelRepr for [T; N] {
    type Scalar = T;
    const CHANNELS: usize = N;
    const ZERO: Self = [T::ZERO; N];
    const ONE: Self = [T::ONE; N];
    type Weights = [Weight; N];
}
impl<T: Scalar> PixelRepr for T {
    type Scalar = Self;
    const CHANNELS: usize = 1;
    const ZERO: Self = Self::ZERO;
    const ONE: Self = Self::ONE;
    type Weights = Weight;
}

/// The building block of the crate, a pixel type that has a `Repr` that defines how this crate represents it
///
/// To access the associated types and constants of your `Repr`, you need to use specifiers like this:
/// ```
/// <<MyPixel as Pixel>::Repr as PixelRepr>::Scalar
/// ```
/// because Rust does not support default associated types yet (in stable).
pub trait Pixel: Sized {
    /// The representation of a pixel as a flat array of scalars with length 1 to 5 (e.g. `[u8; 3]`, `[u16; 4]`, etc.) or a scalar (e.g. `u8`, `f32`, etc.)
    type Repr: PixelRepr;
    /// The default repr to use when creating new images, defaults to [`PixelRepr::ZERO`]
    const DEFAULT: Self::Repr = Self::Repr::ZERO;
}
