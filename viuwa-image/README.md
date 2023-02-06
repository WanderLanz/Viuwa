# viuwa-image

Simple and casual image operations for the [`viuwa`](../README.md) project, mostly for resizing images.

Does not handle color spaces, gamma correction, IO, etc. and is not meant to be a full-featured image processing library.

Any trait or type starting with `Compat` (e.g. `CompatPixelRepr`) is a guaranteed compatibility layer for the [`image`](https://crates.io/crates/image) and [`fast_image_resize`](https://crates.io/crates/fast_image_resize) crates and features.

Side note: `unsafe` usage is limited to explicit buffer allocation write-first optimizations and is not used for any other purpose, so don't get your panties in a bunch.

## Features

- `clap`: Derives `clap::ValueEnum` for `FilterType`
- `image`: Adds APIs for convenient usage with the `image` crate
- `rayon`: Adds APIs for parallel iteration with the `rayon` crate and parallelize resizing
- `fir`: Adds APIs for SIMD optimizations with the `fast_image_resize` crate (requires `CompatPixelRepr` to be implemented for the pixel type)

### Compatibile Pixel Reprs and their `fast_image_resize` equivalents

```rust
u8 => U8,
u16 => U16,
i32 => I32,
f32 => F32,
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
```

## Examples

### Resizing and iterating over an image

```rust
use viuwa_image::{Image, ImageView, ImageOps, Pixel, CompatPixel, FilterType};

// Your arbitrary pixel type
pub struct MyRgbPixel([u8; 3]);

// Implement the `Pixel` trait for your pixel type to use it with `viuwa-image`
impl Pixel for MyRgbPixel {
    type Repr = [u8; 3];
    // Repr is the underlying representation of the pixel, e.g. `[u8; 3]` for RGB
    // Can be any array `[Scalar; N]`
}

// Create an image from a `Vec` of scalar values with 100x100 pixels and 3 channels (RGB)
let mut Ok(image) = Image::<MyRgbPixel>::from_raw(vec![128u8; 100 * 100 * 3], 100, 100) else { unreachable!() };

// Create a new image that is the image resized to within 50x50 pixels
let resized = image.resize(FilterType::Nearest, 50, 50);

// Get a reference to the data of the image
let data: &[u8] = resized.data();
assert_eq!(data.len(), 50 * 50 * 3);

// Create an iterator over the pixels in the image
let pixels = resized.pixels();
assert_eq!(pixels.len(), 50 * 50);

// Create an iterator over the pixel rows in the image
let rows = resized.rows();
assert_eq!(rows.len(), 50);

// Create an iterator over the pixel columns in the image
let columns = resized.columns();
assert_eq!(columns.len(), 50);
```

## Contributing

You are free and welcome to contribute to this project. Please read [CONTRIBUTING.md](../CONTRIBUTING.md) for more information.
