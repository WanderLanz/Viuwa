//! Image filters and kernels
use crate::Weight;

const PI: Weight = ::std::f64::consts::PI as Weight;

// REFERENCE: Cubic
// fn cubic_bc(b: f32, c: f32, x: f32) -> f32 {
//     let x = x.abs();
//     let x = if x < 1. {
//         (12. - 9. * b - 6. * c) * x.powi(3) + (-18. + 12. * b + 6. * c) * x.powi(2) + (6. - 2. * b)
//     } else if x < 2.0 {
//         (-b - 6. * c) * x.powi(3) + (6. * b + 30. * c) * x.powi(2) + (-12. * b - 48. * c) * x + (8. * b + 24. * c)
//     } else {
//         0.0
//     };
//     x / 6.0
// }

#[inline]
pub fn sinc(x: Weight) -> Weight {
    if x == 0. {
        1.
    } else {
        let x = x * PI;
        x.sin() / x
    }
}

/// point kernel
#[inline(always)]
pub fn point_kernel(_: Weight) -> Weight { 1. }

/// average kernel
#[inline]
pub fn box_kernel(x: Weight) -> Weight {
    if x > -0.5 && x <= 0.5 {
        1.0
    } else {
        0.0
    }
}

/// linear kernel
#[inline(always)]
pub fn triangle_kernel(x: Weight) -> Weight { Weight::max(1. - x.abs(), 0.) }

#[inline]
pub fn hamming_kernel(x: Weight) -> Weight {
    let x = x.abs();
    if x == 0. {
        1.
    } else if x >= 1. {
        0.
    } else {
        let x = x * PI;
        (0.54 + 0.46 * x.cos()) * x.sin() / x
    }
}

/// inlined cubic_bc with b=0., c=0.5
#[inline]
pub fn catmull_rom_kernel(x: Weight) -> Weight {
    let x = x.abs();
    if x < 1. {
        const A: Weight = 9. / 6.;
        const B: Weight = 15. / 6.;
        (A * x - B) * x.powi(2) + 1.
    } else if x < 2.0 {
        const A: Weight = 15. / 6.;
        ((-0.5 * x + A) * x - 4.) * x + 2.
    } else {
        0.
    }
}

/// inlined cubic_bc with b=1./3., c=1./3.
#[inline]
pub fn mitchell_netravali_kernel(x: Weight) -> Weight {
    let x = x.abs();
    if x < 1. {
        const A: Weight = 7. / 6.;
        const B: Weight = 16. / 18.;
        (A * x - 2.) * x.powi(2) + B
    } else if x < 2. {
        const A: Weight = -7. / 18.;
        const B: Weight = 20. / 6.;
        const C: Weight = 32. / 18.;
        ((A * x + 2.) * x - B) * x + C
    } else {
        0.
    }
}

// #[inline]
// fn gaussian_kernel(x: Weight) -> Weight { 0.7978846 * (-x.powi(2) / 0.5).exp() }

#[inline]
pub fn lanczos3_kernel(x: Weight) -> Weight {
    if x.abs() < 3. {
        sinc(x) * sinc(x / 3.)
    } else {
        0.
    }
}

/// A specific static filter with a kernel fn and a support radius
#[derive(Clone, Copy)]
pub struct Filter {
    pub kernel: fn(Weight) -> Weight,
    pub support: Weight,
}
pub static FILTER_NEAREST: Filter = Filter { kernel: point_kernel, support: 0. };
pub static FILTER_BOX: Filter = Filter { kernel: box_kernel, support: 0.5 };
pub static FILTER_TRIANGLE: Filter = Filter { kernel: triangle_kernel, support: 1. };
pub static FILTER_HAMMING: Filter = Filter { kernel: hamming_kernel, support: 1. };
pub static FILTER_CATMULL_ROM: Filter = Filter { kernel: catmull_rom_kernel, support: 2. };
pub static FILTER_MITCHELL: Filter = Filter { kernel: mitchell_netravali_kernel, support: 2. };
// pub static FILTER_GAUSSIAN: Filter = Filter { kernel: gaussian_kernel, support: 3. }; // Unused until I can find out how to implement it with fir
pub static FILTER_LANCZOS3: Filter = Filter { kernel: lanczos3_kernel, support: 3. };

/// Dynamic filter type, also implements From<u8>
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum FilterType {
    /// Nearest neighbor filter, no interpolation
    #[default]
    Nearest,
    /// Box filter, also known as average filter
    Box,
    /// Triangle filter, also known as linear filter
    Triangle,
    /// Hamming filter, also known as cosine filter
    Hamming,
    // Catmull-Rom filter, the standard cubic filter
    Catmull,
    /// Mitchell-Netravali filter, a better quality cubic filter
    Mitchell,
    // Gaussian,
    /// Lanczos3 filter, a high quality filter (highest quality we provide)
    Lanczos,
}
use FilterType::*;
impl FilterType {
    /// Get the static filter for this type
    #[inline]
    pub(crate) fn filter(&self) -> Filter {
        match self {
            Nearest => FILTER_NEAREST,
            Box => FILTER_BOX,
            Triangle => FILTER_TRIANGLE,
            Hamming => FILTER_HAMMING,
            Catmull => FILTER_CATMULL_ROM,
            Mitchell => FILTER_MITCHELL,
            // Gaussian => FILTER_GAUSSIAN,
            Lanczos => FILTER_LANCZOS3,
        }
    }
    /// Get the static convolution algorithm for this type
    #[cfg(feature = "fir")]
    #[inline]
    pub(crate) fn algorithm(&self) -> ::fast_image_resize::ResizeAlg {
        use ::fast_image_resize::{FilterType as F, ResizeAlg as A};
        match self {
            Nearest => A::Nearest,
            Box => A::Convolution(F::Box),
            Triangle => A::Convolution(F::Bilinear),
            Hamming => A::Convolution(F::Hamming),
            Catmull => A::Convolution(F::CatmullRom),
            Mitchell => A::Convolution(F::Mitchell),
            Lanczos => A::Convolution(F::Lanczos3),
        }
    }
    /// Get the static supersampling algorithm for this type
    #[cfg(feature = "fir")]
    #[inline]
    pub(crate) fn ss_algorithm(&self, multiplicity: u8) -> ::fast_image_resize::ResizeAlg {
        use ::fast_image_resize::{FilterType as F, ResizeAlg as A};
        match self {
            Nearest => A::Nearest,
            Box => A::SuperSampling(F::Box, multiplicity),
            Triangle => A::SuperSampling(F::Bilinear, multiplicity),
            Hamming => A::SuperSampling(F::Hamming, multiplicity),
            Catmull => A::SuperSampling(F::CatmullRom, multiplicity),
            Mitchell => A::SuperSampling(F::Mitchell, multiplicity),
            Lanczos => A::SuperSampling(F::Lanczos3, multiplicity),
        }
    }
    /// Cycle to the next filter type, provided for convenience
    #[inline]
    pub fn cycle(&self) -> FilterType {
        match self {
            Nearest => Box,
            Box => Triangle,
            Triangle => Hamming,
            Hamming => Catmull,
            Catmull => Mitchell,
            Mitchell => Lanczos,
            // Gaussian => Lanczos,
            Lanczos => Nearest,
        }
    }
}
impl From<u8> for FilterType {
    fn from(i: u8) -> Self {
        if i > 6 {
            Nearest
        } else {
            unsafe { ::core::mem::transmute(i) }
        }
    }
}
