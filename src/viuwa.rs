use crate::{Args, BoxResult};

use std::io::{self, stdout, StdoutLock, Write};
#[cfg(target_family = "wasm")]
use std::io::{stdin, Read};

use image::{imageops::FilterType, DynamicImage};
#[cfg(feature = "rayon-resizer")]
use image::{ImageBuffer, Luma, Pixel, Rgb};

pub mod ansi;

use ansi::{AnsiImageBuffer, TerminalImpl};

/// Static ref to resize function for convenience and cleaner code
#[cfg(feature = "rayon-resizer")]
static RESIZE_FN: &'static (dyn Fn(&DynamicImage, u32, u32, FilterType) -> DynamicImage + Sync) = &unstable_rayon::resize;
#[cfg(not(feature = "rayon-resizer"))]
static RESIZE_FN: &'static (dyn Fn(&DynamicImage, u32, u32, FilterType) -> DynamicImage + Sync) =
        &image::DynamicImage::resize;

/// Wrapper around possibly user-controlled color attributes
#[derive(Debug, Clone, Copy)]
pub struct ColorAttributes {
        /// luma correct as a color distance threshold
        pub luma_correct: u32,
}

impl ColorAttributes {
        /// luma correct is 0..=100, 100 is the highest luma correct
        // distance threshold = (MAX_COLOR_DISTANCE / 100) * ((100 - luma_correct)^2 / 100)
        pub fn new(luma_correct: u32) -> Self {
                Self {
                        luma_correct: (((100 - luma_correct).pow(2) / 100) as f32 * ansi::color::MAP_0_100_DIST) as u32,
                }
        }
}

#[derive(Debug, Clone, Copy)]
pub enum ColorType {
        Color,
        Color256,
        Gray,
        Gray256,
}

impl ColorType {
        pub fn cycle(&self) -> ColorType {
                match self {
                        ColorType::Color => ColorType::Color256,
                        ColorType::Color256 => ColorType::Gray,
                        ColorType::Gray => ColorType::Gray256,
                        ColorType::Gray256 => ColorType::Color,
                }
        }
        pub fn is_color(&self) -> bool {
                match self {
                        ColorType::Color | ColorType::Color256 => true,
                        ColorType::Gray | ColorType::Gray256 => false,
                }
        }
        // pub fn is_8bit(&self) -> bool {
        //         match self {
        //                 ColorType::Color256 | ColorType::Gray256 => true,
        //                 ColorType::Color | ColorType::Gray => false,
        //         }
        // }
        pub fn is_24bit(&self) -> bool {
                match self {
                        ColorType::Color | ColorType::Gray => true,
                        ColorType::Color256 | ColorType::Gray256 => false,
                }
        }
}
// /// For when and if we decide to add more TUI features and want to abstract away the cli args
// pub struct DynamicVars {
//         pub color_type: ColorType,
//         pub color_attrs: ColorAttributes,
//         pub filter: FilterType,
// }

pub struct Viuwa<'a> {
        pub orig: DynamicImage,
        pub buf: AnsiImageBuffer,
        pub size: (u16, u16),
        pub lock: StdoutLock<'a>,
        pub args: Args,
}

impl<'a> Viuwa<'a> {
        /// Create a new viuwa instance
        pub fn new(orig: DynamicImage, args: Args) -> BoxResult<Self> {
                let mut lock = stdout().lock();
                let size = lock.size(args.quiet)?;
                let buf = AnsiImageBuffer::from(
                        RESIZE_FN(&orig, size.0 as u32, size.1 as u32 * 2, args.filter),
                        &args.color,
                        &ColorAttributes::new(args.luma_correct),
                );
                Ok(Self {
                        orig,
                        buf,
                        size,
                        lock,
                        args,
                })
        }
        // seperate spawns because of resize event
        /// Start viuwa app
        #[cfg(any(unix, windows))]
        pub fn spawn(mut self) -> BoxResult<()> {
                use crossterm::event::{Event, KeyCode, KeyEventKind};
                self.lock.enable_raw_mode()?;
                self.lock.write_all(
                        [
                                ansi::term::ENTER_ALT_SCREEN,
                                ansi::term::HIDE_CURSOR,
                                ansi::term::DISABLE_LINE_WRAP,
                                ansi::term::CLEAR_BUFFER,
                        ]
                        .concat()
                        .as_bytes(),
                )?;
                self.lock.flush()?;
                self._draw()?;
                loop {
                        match crossterm::event::read()? {
                                Event::Key(e) if e.kind == KeyEventKind::Press => match e.code {
                                        KeyCode::Char('q') | KeyCode::Esc => break,
                                        KeyCode::Char('r') => {
                                                self._draw()?;
                                        }
                                        KeyCode::Char('h') => {
                                                self._help()?;
                                                self._draw()?;
                                        }
                                        KeyCode::Char('f') => {
                                                self._cycle_filter();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('c') => {
                                                self._cycle_color();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('1') => {
                                                self.args.color = ColorType::Color;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('2') => {
                                                self.args.color = ColorType::Color256;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('3') => {
                                                self.args.color = ColorType::Gray;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('4') => {
                                                self.args.color = ColorType::Gray256;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('!') => {
                                                self.args.filter = FilterType::Nearest;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('@') => {
                                                self.args.filter = FilterType::Triangle;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('#') => {
                                                self.args.filter = FilterType::CatmullRom;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('$') => {
                                                self.args.filter = FilterType::Gaussian;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        KeyCode::Char('%') => {
                                                self.args.filter = FilterType::Lanczos3;
                                                self._rebuild_buf();
                                                self._draw()?;
                                        }
                                        _ => {}
                                },
                                Event::Resize(w, h) => {
                                        self._handle_resize(w, h);
                                        self._draw()?;
                                }
                                _ => (),
                        }
                }
                self.lock.write_all(
                        [
                                ansi::term::ENABLE_LINE_WRAP,
                                ansi::term::SHOW_CURSOR,
                                ansi::term::EXIT_ALT_SCREEN,
                        ]
                        .concat()
                        .as_bytes(),
                )?;
                self.lock.disable_raw_mode()?;
                self.lock.flush()?;
                Ok(())
        }
        /// Start viuwa app
        #[cfg(target_family = "wasm")]
        pub fn spawn(mut self) -> BoxResult<()> {
                self.lock.enable_raw_mode()?;
                self.lock.write_all(
                        [
                                ansi::term::ENTER_ALT_SCREEN,
                                ansi::term::HIDE_CURSOR,
                                ansi::term::DISABLE_LINE_WRAP,
                        ]
                        .concat()
                        .as_bytes(),
                )?;
                self.lock.flush()?;
                self._draw()?;
                let mut buf = [0; 1];
                loop {
                        stdin().read_exact(&mut buf)?;
                        match buf[0] {
                                b'q' => break,
                                b'r' => {
                                        let size = self.lock.size(self.args.quiet)?;
                                        self._handle_resize(size.0, size.1);
                                        self._draw()?;
                                }
                                b'h' => {
                                        self._help()?;
                                        self._draw()?;
                                }
                                b'f' => {
                                        self._cycle_filter();
                                        self._draw()?;
                                }
                                b'c' => {
                                        self._cycle_color();
                                        self._draw()?;
                                }
                                b'1' => {
                                        self.args.color = ColorType::Color;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'2' => {
                                        self.args.color = ColorType::Color256;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'3' => {
                                        self.args.color = ColorType::Gray;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'4' => {
                                        self.args.color = ColorType::Gray256;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'!' => {
                                        self.args.filter = FilterType::Nearest;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'@' => {
                                        self.args.filter = FilterType::Triangle;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'#' => {
                                        self.args.filter = FilterType::CatmullRom;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'$' => {
                                        self.args.filter = FilterType::Gaussian;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                b'%' => {
                                        self.args.filter = FilterType::Lanczos3;
                                        self._rebuild_buf();
                                        self._draw()?;
                                }
                                _ => {}
                        }
                }
                self.lock.write_all(
                        [
                                ansi::term::ENABLE_LINE_WRAP,
                                ansi::term::SHOW_CURSOR,
                                ansi::term::EXIT_ALT_SCREEN,
                        ]
                        .concat()
                        .as_bytes(),
                )?;
                self.lock.disable_raw_mode()?;
                self.lock.flush()?;
                Ok(())
        }
        /// Write the buffer to the terminal, and move the cursor to the bottom left
        fn _draw(&mut self) -> BoxResult<()> {
                self.lock.clear_screen()?;
                let ox = (self.size.0 - self.buf.size.0) / 2;
                let oy = (self.size.1 - self.buf.size.1) / 2;
                for (y, line) in self.buf.buf.iter().enumerate() {
                        self.lock.cursor_to(ox, oy + y as u16)?;
                        self.lock.write_all(line.0.as_bytes())?;
                        self.lock.attr_reset()?;
                }
                self.lock.cursor_to(0, self.size.1)?;
                self.lock.flush()?;
                Ok(())
        }
        /// clear screen, print help, and quit 'q'
        fn _help(&mut self) -> BoxResult<()> {
                self.lock.clear_screen()?;
                self.lock.cursor_home()?;
                self._write_centerx(0, "Viuwa help:")?;
                self._write_centerxy_align_all(
                        &[
                                "[q]: quit",
                                "[r]: redraw",
                                "[h]: help",
                                "[c]: cycle color",
                                "[f]: cycle filter",
                                "[1]: set color to Truecolor",
                                "[2]: set color to 256",
                                "[3]: set color to Gray",
                                "[4]: set color to 256Gray",
                                "[Shift + 1]: set filter to nearest",
                                "[Shift + 2]: set filter to triangle",
                                "[Shift + 3]: set filter to catmull rom",
                                "[Shift + 4]: set filter to gaussian",
                                "[Shift + 5]: set filter to lanczos3",
                        ]
                        .to_vec(),
                )?;
                self.lock.cursor_to(0, self.size.1)?;
                self.lock.flush()?;
                wait_for_quit()
        }
        /// handle resize event
        fn _handle_resize(&mut self, w: u16, h: u16) {
                let nsz = (w + 1, h + 1);
                if nsz != self.size {
                        self.size = nsz;
                        self._rebuild_buf();
                }
        }
        /// print a string centered on the x axis
        fn _write_centerx(&mut self, y: u16, s: &str) -> io::Result<()> {
                self.lock.cursor_to((self.size.0 - s.len() as u16) / 2, y)?;
                self.lock.write_all(s.as_bytes())?;
                Ok(())
        }
        /// print strings centered and aligned on the x axis and y axis
        fn _write_centerxy_align_all(&mut self, s: &Vec<&str>) -> BoxResult<()> {
                if let Some(max) = s.into_iter().map(|x| x.len()).max() {
                        let ox = (self.size.0 - max as u16) / 2;
                        let oy = (self.size.1 - s.len() as u16) / 2;
                        for (i, line) in s.into_iter().enumerate() {
                                self.lock.cursor_to(ox, oy + i as u16)?;
                                self.lock.write_all(line.as_bytes())?;
                        }
                        Ok(())
                } else {
                        Err("No strings to write".into())
                }
        }
        /// Cycle through filter types, and rebuild buffer
        fn _cycle_filter(&mut self) {
                self.args.filter = match self.args.filter {
                        FilterType::Nearest => FilterType::Triangle,
                        FilterType::Triangle => FilterType::CatmullRom,
                        FilterType::CatmullRom => FilterType::Gaussian,
                        FilterType::Gaussian => FilterType::Lanczos3,
                        FilterType::Lanczos3 => FilterType::Nearest,
                };
                self._rebuild_buf();
        }
        /// Cycle through output formats, and rebuild buffer
        fn _cycle_color(&mut self) {
                self.args.color = self.args.color.cycle();
                self._rebuild_buf();
        }
        /// Rebuild the buffer with the current image, filter, and format
        fn _rebuild_buf(&mut self) {
                self.buf.replace_image(
                        RESIZE_FN(&self.orig, self.size.0 as u32, self.size.1 as u32 * 2, self.args.filter),
                        &self.args.color,
                        &ColorAttributes::new(self.args.luma_correct),
                );
        }
}

#[cfg(target_family = "wasm")]
fn wait_for_quit() -> BoxResult<()> {
        let mut buf = [0; 1];
        let mut stdin = stdin().lock();
        loop {
                stdin.read_exact(&mut buf)?;
                match buf[0] {
                        b'q' => break,
                        _ => (),
                }
        }
        Ok(())
}
#[cfg(any(windows, unix))]
fn wait_for_quit() -> BoxResult<()> {
        loop {
                match crossterm::event::read()? {
                        crossterm::event::Event::Key(crossterm::event::KeyEvent {
                                code: crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc,
                                ..
                        }) => break,
                        _ => continue,
                }
        }
        Ok(())
}

/// Print ANSI image to stdout without attempting to use alternate screen buffer or other fancy stuff
pub fn inline(orig: DynamicImage, args: Args) -> BoxResult<()> {
        let size = match (args.width, args.height) {
                (None, None) => stdout().size(args.quiet)?,
                (None, Some(h)) => (crate::MAX_COLS, h),
                (Some(w), None) => (w, crate::MAX_ROWS),
                (Some(w), Some(h)) => (w, h),
        };
        let buf = AnsiImageBuffer::from(
                RESIZE_FN(&orig, size.0 as u32, size.1 as u32 * 2, args.filter),
                &args.color,
                &ColorAttributes::new(args.luma_correct),
        );
        let mut lock = stdout().lock();
        for line in buf.buf.iter() {
                lock.write_all(line.0.as_bytes())?;
                lock.attr_reset()?;
                lock.write_all(b"\n")?;
        }
        lock.flush()?;
        Ok(())
}

#[cfg(feature = "rayon-resizer")]
/// Parallelized [image] functions with [rayon], using [ndarray] when needed
mod unstable_rayon {
        #![allow(non_upper_case_globals)]

        use super::*;

        use rayon::prelude::*;

        // sinc function: the ideal sampling filter.
        fn sinc(t: f32) -> f32 {
                if t == 0.0 {
                        1.0
                } else {
                        let a = t * std::f32::consts::PI;
                        a.sin() / a
                }
        }

        #[inline]
        fn clamp<N>(a: N, min: N, max: N) -> N
        where
                N: PartialOrd,
        {
                if a < min {
                        min
                } else if a > max {
                        max
                } else {
                        a
                }
        }

        const nearest_support: f32 = 0.0;
        fn nearest_kernel(_: f32) -> f32 { 1.0 }
        const triangle_support: f32 = 1.0;
        fn triangle_kernel(x: f32) -> f32 {
                if x.abs() < 1.0 {
                        1.0 - x.abs()
                } else {
                        0.0
                }
        }
        const catmull_rom_support: f32 = 2.0;
        fn catmull_rom_kernel(x: f32) -> f32 {
                let a = x.abs();
                let k = if a < 1.0 {
                        9.0 * a.powi(3) - 15.0 * a.powi(2) + 6.0
                } else if a < 2.0 {
                        -3.0 * a.powi(3) + 15.0 * a.powi(2) - 24.0 * a + 12.0
                } else {
                        0.0
                };
                k / 6.0
        }
        const gaussian_support: f32 = 3.0;
        fn gaussian_kernel(x: f32) -> f32 { 0.7978846 * (-x.powi(2) / 0.5).exp() }
        const lanczos3_support: f32 = 3.0;
        fn lanczos3_kernel(x: f32) -> f32 {
                let t = 3.0;
                if x.abs() < t {
                        sinc(x) * sinc(x / t)
                } else {
                        0.0
                }
        }

        /// Resize an image to given size, aspect ratio preserving, same as [image] crate... except parallelized with [rayon] (and [ndarray] for column-major image mutations)
        pub fn resize(img: &DynamicImage, nw: u32, nh: u32, filter: FilterType) -> DynamicImage {
                let (w, h) = image::GenericImageView::dimensions(img);
                // find aspect ratio preserving size for new image
                let (nw, nh) = {
                        let wratio = nw as f64 / w as f64;
                        let hratio = nh as f64 / h as f64;

                        let ratio = f64::min(wratio, hratio);

                        let nw = u64::max((w as f64 * ratio).round() as u64, 1);
                        let nh = u64::max((h as f64 * ratio).round() as u64, 1);

                        if nw > u64::from(u32::MAX) {
                                let ratio = u32::MAX as f64 / w as f64;
                                (u32::MAX, u32::max((h as f64 * ratio).round() as u32, 1))
                        } else if nh > u64::from(u32::MAX) {
                                let ratio = u32::MAX as f64 / h as f64;
                                (u32::max((w as f64 * ratio).round() as u32, 1), u32::MAX)
                        } else {
                                (nw as u32, nh as u32)
                        }
                };
                // don't resize if the image is already the correct size
                if (w, h) == (nw, nh) {
                        return img.clone();
                }
                // else we inline the entire resize function to optimize with parallelism and cache
                let (kernel, support): (&'static (dyn Fn(f32) -> f32 + Sync), f32) = match filter {
                        FilterType::Nearest => (&nearest_kernel, nearest_support),
                        FilterType::Triangle => (&triangle_kernel, triangle_support),
                        FilterType::CatmullRom => (&catmull_rom_kernel, catmull_rom_support),
                        FilterType::Gaussian => (&gaussian_kernel, gaussian_support),
                        FilterType::Lanczos3 => (&lanczos3_kernel, lanczos3_support),
                };
                match img {
                        DynamicImage::ImageRgb8(ref p) => DynamicImage::ImageRgb8(horizontal_sample::<
                                Rgb<f32>,
                                Rgb<u8>,
                                { Rgb::<u8>::CHANNEL_COUNT as usize },
                        >(
                                &vertical_sample::<Rgb<u8>, Rgb<f32>, { Rgb::<f32>::CHANNEL_COUNT as usize }>(
                                        p, nh, kernel, support,
                                ),
                                nw,
                                kernel,
                                support,
                        )),
                        DynamicImage::ImageLuma8(ref p) => DynamicImage::ImageLuma8(horizontal_sample::<
                                Luma<f32>,
                                Luma<u8>,
                                { Luma::<u8>::CHANNEL_COUNT as usize },
                        >(
                                &vertical_sample::<Luma<u8>, Luma<f32>, { Luma::<f32>::CHANNEL_COUNT as usize }>(
                                        p, nh, kernel, support,
                                ),
                                nw,
                                kernel,
                                support,
                        )),
                        _ => unreachable!(),
                }
        }

        fn vertical_sample<IP: Pixel<Subpixel = u8> + Sync, OP: Pixel<Subpixel = f32>, const CHANNEL_COUNT: usize>(
                image: &ImageBuffer<IP, Vec<u8>>,
                new_height: u32,
                kernel: &'static (dyn Fn(f32) -> f32 + Sync),
                support: f32,
        ) -> ImageBuffer<OP, Vec<f32>> {
                let (width, height) = image.dimensions();
                let mut new_image = ImageBuffer::<OP, Vec<f32>>::new(width, new_height).into_vec();
                let ratio = height as f32 / new_height as f32;
                let sratio = if ratio < 1.0 { 1.0 } else { ratio };
                let src_support = support * sratio;
                (0..new_height)
                        .into_par_iter()
                        .zip(new_image.par_chunks_exact_mut(width as usize * CHANNEL_COUNT))
                        .for_each_with(image, |image, (outy, row)| {
                                let inputy = (outy as f32 + 0.5) * ratio;
                                let left = (inputy - src_support).floor() as i64;
                                let left = clamp(left, 0, <i64 as From<_>>::from(height) - 1) as u32;
                                let right = (inputy + src_support).ceil() as i64;
                                let right = clamp(right, <i64 as From<_>>::from(left) + 1, <i64 as From<_>>::from(height))
                                        as u32;
                                let inputy = inputy - 0.5;
                                let mut weights = Vec::with_capacity(right.saturating_sub(left) as usize);
                                let mut sum = 0.0;
                                for i in left..right {
                                        let w = kernel((i as f32 - inputy) / sratio);
                                        weights.push(w);
                                        sum += w;
                                }
                                weights.iter_mut().for_each(|w| *w /= sum);
                                for x in 0..width {
                                        let row_index = x as usize * CHANNEL_COUNT;
                                        for (i, w) in weights.iter().enumerate() {
                                                let p = image.get_pixel(x, left + i as u32).channels();
                                                for i in 0..CHANNEL_COUNT {
                                                        row[row_index + i] += p[i] as f32 * w;
                                                }
                                        }
                                }
                        });
                ImageBuffer::from_vec(width, new_height, new_image).expect("Everything. Is. Fine. :VSRGB")
        }

        fn horizontal_sample<IP: Pixel<Subpixel = f32> + Sync, OP: Pixel<Subpixel = u8>, const CHANNEL_COUNT: usize>(
                image: &ImageBuffer<IP, Vec<f32>>,
                new_width: u32,
                kernel: &'static (dyn Fn(f32) -> f32 + Sync),
                support: f32,
        ) -> ImageBuffer<OP, Vec<u8>> {
                let (width, height) = image.dimensions();
                // Axes: rows, columns, pixels
                let mut new_image = unsafe {
                        ndarray::Array3::from_shape_vec_unchecked(
                                (height as usize, new_width as usize, CHANNEL_COUNT),
                                ImageBuffer::<OP, Vec<u8>>::new(new_width, height).into_vec(),
                        )
                };
                let max: f32 = u8::MAX as f32;
                let min: f32 = u8::MIN as f32;
                let ratio = width as f32 / new_width as f32;
                let sratio = if ratio < 1.0 { 1.0 } else { ratio };
                let src_support = support * sratio;
                new_image
                        .axis_iter_mut(ndarray::Axis(1)) // Iterate over the columns
                        .into_par_iter()
                        .enumerate()
                        .for_each_with(image, |image, (outx, mut col)| {
                                let inputx = (outx as f32 + 0.5) * ratio;
                                let left = (inputx - src_support).floor() as i64;
                                let left = clamp(left, 0, width as i64 - 1) as u32;
                                let right = (inputx + src_support).ceil() as i64;
                                let right = clamp(right, left as i64 + 1, width as i64) as u32;
                                let inputx = inputx - 0.5;
                                // Allocating new vector because we rebuild vector of weights for each column
                                let mut weights = Vec::with_capacity(right.saturating_sub(left) as usize);
                                let mut sum = 0.0;
                                for i in left..right {
                                        let w = kernel((i as f32 - inputx) / sratio);
                                        weights.push(w);
                                        sum += w;
                                }
                                weights.iter_mut().for_each(|w| *w /= sum);
                                for y in 0..height {
                                        let mut t = [0.0; CHANNEL_COUNT];
                                        for (i, w) in weights.iter().enumerate() {
                                                let p = image.get_pixel(left + i as u32, y).channels();
                                                for i in 0..CHANNEL_COUNT {
                                                        t[i] += p[i] * w;
                                                }
                                        }
                                        col.index_axis_mut(ndarray::Axis(0), y as usize) // Get pixel at row[y]
                                                .into_iter()
                                                .zip(t.iter())
                                                .for_each(|(p, &t)| {
                                                        *p = clamp(t, min, max).round() as u8;
                                                });
                                }
                        });
                ImageBuffer::from_vec(new_width, height, new_image.into_raw_vec()).expect("Everything. Is. Fine. :HSRGB")
        }
}
