#[cfg(not(feature = "fir"))]
mod filter {
    use std::f32::consts::PI;
    #[inline]
    fn sinc(x: f32) -> f32 {
        if x == 0. {
            1.
        } else {
            let x = x * PI;
            x.sin() / x
        }
    }
    #[inline(always)]
    fn point_kernel(_: f32) -> f32 { 1. }
    #[inline]
    fn box_kernel(x: f32) -> f32 {
        if x > -0.5 && x <= 0.5 {
            1.0
        } else {
            0.0
        }
    }
    #[inline(always)]
    fn triangle_kernel(x: f32) -> f32 { f32::max(1. - x.abs(), 0.) }
    #[inline]
    fn hamming_kernel(x: f32) -> f32 {
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
    fn catmull_rom_kernel(x: f32) -> f32 {
        let x = x.abs();
        if x < 1. {
            const A: f32 = 9. / 6.;
            const B: f32 = 15. / 6.;
            (A * x - B) * x.powi(2) + 1.
        } else if x < 2.0 {
            const A: f32 = 15. / 6.;
            ((-0.5 * x + A) * x - 4.) * x + 2.
        } else {
            0.
        }
    }
    /// inlined cubic_bc with b=1./3., c=1./3.
    #[inline]
    fn mitchell_netravali_kernel(x: f32) -> f32 {
        let x = x.abs();
        if x < 1. {
            const A: f32 = 7. / 6.;
            const B: f32 = 16. / 18.;
            (A * x - 2.) * x.powi(2) + B
        } else if x < 2. {
            const A: f32 = -7. / 18.;
            const B: f32 = 20. / 6.;
            const C: f32 = 32. / 18.;
            ((A * x + 2.) * x - B) * x + C
        } else {
            0.
        }
    }
    #[inline]
    fn gaussian_kernel(x: f32) -> f32 { 0.7978846 * (-x.powi(2) / 0.5).exp() }
    #[inline]
    fn lanczos3_kernel(x: f32) -> f32 {
        if x.abs() < 3. {
            sinc(x) * sinc(x / 3.)
        } else {
            0.
        }
    }
    // for reference
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

    /// A specific static filter with a kernel fn and a support radius
    #[derive(Clone, Copy)]
    pub struct Filter {
        pub kernel: fn(f32) -> f32,
        pub support: f32,
    }
    pub static FILTER_NEAREST: Filter = Filter { kernel: point_kernel, support: 0. };
    pub static FILTER_BOX: Filter = Filter { kernel: box_kernel, support: 0.5 };
    pub static FILTER_TRIANGLE: Filter = Filter { kernel: triangle_kernel, support: 1. };
    pub static FILTER_HAMMING: Filter = Filter { kernel: hamming_kernel, support: 1. };
    pub static FILTER_CATMULL_ROM: Filter = Filter { kernel: catmull_rom_kernel, support: 2. };
    pub static FILTER_MITCHELL: Filter = Filter { kernel: mitchell_netravali_kernel, support: 2. };
    pub static FILTER_GAUSSIAN: Filter = Filter { kernel: gaussian_kernel, support: 3. };
    pub static FILTER_LANCZOS3: Filter = Filter { kernel: lanczos3_kernel, support: 3. };
}
#[cfg(not(feature = "fir"))]
pub use filter::*;

/// Dynamic filter type
#[cfg(not(feature = "fir"))]
#[cfg_attr(feature = "clap", clap::ValueEnum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterType {
    #[default]
    Nearest,
    Box,
    Triangle,
    Hamming,
    Catmull,
    Mitchell,
    Gaussian,
    Lanczos,
}
/// Dynamic filter type
#[cfg(feature = "fir")]
#[cfg_attr(feature = "clap", clap::ValueEnum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterType {
    #[default]
    Nearest,
    Box,
    Triangle,
    Hamming,
    Catmull,
    Mitchell,
    Lanczos,
}
use FilterType::*;
#[cfg(not(feature = "fir"))]
impl FilterType {
    #[inline]
    pub fn filter(&self) -> &Filter {
        match self {
            Nearest => &FILTER_NEAREST,
            Box => &FILTER_BOX,
            Triangle => &FILTER_TRIANGLE,
            Hamming => &FILTER_HAMMING,
            Catmull => &FILTER_CATMULL_ROM,
            Mitchell => &FILTER_MITCHELL,
            Gaussian => &FILTER_GAUSSIAN,
            Lanczos => &FILTER_LANCZOS3,
        }
    }
    #[inline]
    pub fn cycle(&self) -> FilterType {
        match self {
            Nearest => Box,
            Box => Triangle,
            Triangle => Hamming,
            Hamming => Catmull,
            Catmull => Mitchell,
            Mitchell => Gaussian,
            Gaussian => Lanczos,
            Lanczos => Nearest,
        }
    }
}
#[cfg(not(feature = "fir"))]
impl From<u8> for FilterType {
    fn from(i: u8) -> Self {
        match i {
            0 => Nearest,
            1 => Box,
            2 => Triangle,
            3 => Hamming,
            4 => Catmull,
            5 => Mitchell,
            6 => Gaussian,
            7 => Lanczos,
            _ => Nearest,
        }
    }
}
#[cfg(feature = "fir")]
impl FilterType {
    #[inline]
    pub fn filter(&self) -> ::fast_image_resize::ResizeAlg {
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
    #[inline]
    pub fn cycle(&self) -> FilterType {
        match self {
            Nearest => Box,
            Box => Triangle,
            Triangle => Hamming,
            Hamming => Catmull,
            Catmull => Mitchell,
            Mitchell => Lanczos,
            Lanczos => Nearest,
        }
    }
}
#[cfg(feature = "fir")]
impl From<u8> for FilterType {
    fn from(i: u8) -> Self {
        match i {
            0 => Nearest,
            1 => Box,
            2 => Triangle,
            3 => Hamming,
            4 => Catmull,
            5 => Mitchell,
            6 => Lanczos,
            _ => Nearest,
        }
    }
}
