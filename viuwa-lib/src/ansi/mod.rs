//! trying my best to make an pure ANSI module for all platforms...
//!
//! refs:
//!  - vt100:
//!     - https://vt100.net/docs/vt100-ug/contents.html
//!  - fnky's gist:
//!     - https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
//!  - xterm:
//!     - https://www.xfree86.org/current/ctlseqs.html
//!     - https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
//!  - windows:
//!     - https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences
//!  - linux:
//!     - https://en.wikipedia.org/wiki/ANSI_escape_code
//!  - iterm:
//!     - https://iterm2.com/documentation-escape-codes.html
//!     - https://chromium.googlesource.com/apps/libapps/+/master/hterm/doc/ControlSequences.md#OSC-1337
//!  - kitty:
//!     - https://sw.kovidgoyal.net/kitty/graphics-protocol.html
//!  - alacritty:
//!     - https://github.com/alacritty/alacritty/blob/master/docs/escape_support.md
//!  - mintty:
//!     - https://github.com/mintty/mintty/wiki/CtrlSeqs
//!  - sixel:
//!     - https://en.wikipedia.org/wiki/Sixel
//!     - https://konfou.xyz/posts/sixel-for-terminal-graphics
//!  - sixel spec:
//!     - https://vt100.net/docs/vt510-rm/sixel.html
//!  - 256 colors:
//!    - https://robotmoon.com/256-colors
//!
//! for reference:
//!  - ESC = escape = "\x1B"
//!  - ST = string terminator = "\x1B\\"
//!  - CSI = control sequence introducer = "\x1B["
//!  - OSC = operating system command = "\x1B]"
//!  - DCS = device control string = "\x1BP"
//!  - APM = application program mode = "\x1B_"
//!  - SGR = select graphic rendition = "\x1B[" + _ + "m"

// Maybe make a PR for crossterm to add pure ansi backup when we aren't compiling for UNIX/Windows?

#![allow(dead_code)]

use ::std::io;
use color::{AnsiColor, PixelWriter, RawPixel};

use super::*;

#[macro_use]
pub mod macros;

pub mod consts;
use consts::*;
pub mod traits;
pub use traits::*;

pub mod color;

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

#[derive(Debug, Clone)]
/// A single row of an ansi image, representing 2 rows of the real image pixels
pub struct AnsiRow(pub Vec<u8>);
impl AnsiRow {
    #[inline(always)]
    pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional) }
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self { Self(Vec::with_capacity(capacity)) }
    #[inline(always)]
    pub fn clear(&mut self) { self.0.clear() }
    #[inline(always)]
    pub fn reserve_color<C: AnsiColor>(&mut self, additional: usize) {
        self.0.reserve(additional * <C::Writer as color::AnsiColorWriter>::RESERVE_SIZE)
    }
    #[inline]
    pub fn with_capacity_color<C: AnsiColor>(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity * <C::Writer as color::AnsiColorWriter>::RESERVE_SIZE))
    }
    #[inline]
    pub fn extend_fgs_bgs<P: RawPixel, C: AnsiColor>(
        &mut self,
        fgs: ArrayView2<<P::Repr as PixelRepr>::Scalar>,
        bgs: ArrayView2<<P::Repr as PixelRepr>::Scalar>,
        attrs: &ColorAttributes,
    ) {
        // unsafe { *(fg.as_ptr() as *const P::Repr) } *should* be safe to do as long as we ensure that the array is the correct size
        fgs.outer_iter().zip(bgs.outer_iter()).for_each(|(fg, bg)| {
            PixelWriter::fg::<P, C>(&mut self.0, unsafe { *(fg.as_ptr().cast()) }, attrs);
            PixelWriter::bg::<P, C>(&mut self.0, unsafe { *(bg.as_ptr().cast()) }, attrs);
            self.0.extend(crate::UPPER_HALF_BLOCK.as_bytes());
        });
    }
    #[inline]
    pub fn extend_fgs<P: RawPixel, C: AnsiColor>(
        &mut self,
        fgs: ArrayView2<<P::Repr as PixelRepr>::Scalar>,
        attrs: &ColorAttributes,
    ) {
        fgs.outer_iter().for_each(|fg| {
            PixelWriter::fg::<P, C>(&mut self.0, unsafe { *(fg.as_ptr().cast()) }, attrs);
            self.0.extend(crate::UPPER_HALF_BLOCK.as_bytes());
        });
    }
    #[inline]
    pub fn as_slice(&self) -> &[u8] { &self.0 }
}

/// An ansi image, 2 rows of pixels per row of ansi
/// "ratchet" memory buffer, so that we minimize memory allocations, because we *should* be relatively memory efficient
// NOTE: A 2D Vec<AnsiRow> is necessary to avoid extreme complexity and manually handling our Vec lengths and such, and it saves some memory too
#[derive(Debug, Clone)]
pub struct AnsiImage {
    buf: Vec<AnsiRow>,
    size: (u16, u16),
}

macro_rules! match_color_as_C {
    ($col: expr, $f: block) => {
        match $col {
            ColorType::Color256 => {
                type C = color::Color256;
                $f
            }
            ColorType::Color => {
                type C = color::ColorRgb;
                $f
            }
            ColorType::Gray256 => {
                type C = color::Gray256;
                $f
            }
            ColorType::Gray => {
                type C = color::GrayRgb;
                $f
            }
        }
    };
}

impl AnsiImage {
    /// Create a new AnsiImageBuffer from a given image and color type and attributes.
    #[instrument(skip_all)]
    pub fn new<P: RawPixel>(img: &ImageBuffer<P, Vec<u8>>, color_type: &ColorType, color_attrs: &ColorAttributes) -> Self {
        let size = Self::_get_size_from(img.dimensions());
        let buf = Vec::with_capacity(size.1 as usize);
        let mut ret = Self { buf, size };
        match_color_as_C!(color_type, {
            let reserve = size.0 as usize * <<C as AnsiColor>::Writer as color::AnsiColorWriter>::RESERVE_SIZE;
            ret.buf.resize_with(size.1 as usize, || AnsiRow::with_capacity(reserve));
            ret._fill::<P, C>(ImageView::from(img), color_attrs);
        });
        ret
    }
    /// Replace image in buffer with new image, assumes image is resized to fit.
    #[instrument(skip(self, img))]
    pub fn replace_image<P: RawPixel>(
        &mut self,
        img: &ImageBuffer<P, Vec<u8>>,
        color_type: &ColorType,
        color_attrs: &ColorAttributes,
    ) {
        self.size = Self::_get_size_from(img.dimensions());
        match_color_as_C!(color_type, {
            self._pour::<C>();
            self._fill::<P, C>(ImageView::from(img), color_attrs);
        });
    }
    #[inline]
    pub fn size(&self) -> &(u16, u16) { &self.size }
    #[inline]
    pub fn rows(&self) -> core::slice::Iter<AnsiRow> {
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr(), self.size.1 as usize) }.iter()
    }
    // No reason to expose this.
    fn rows_mut(&mut self) -> core::slice::IterMut<AnsiRow> {
        unsafe { core::slice::from_raw_parts_mut(self.buf.as_mut_ptr(), self.size.1 as usize) }.iter_mut()
    }
    /// Clear each row within size and reserve more space as needed.
    fn _pour<C: AnsiColor>(&mut self) {
        // clear and reserve rows already initialized within size
        let lines = self.size.1 as usize;
        let res = self.size.0 as usize * <C::Writer as color::AnsiColorWriter>::RESERVE_SIZE;
        self.buf.iter_mut().take(lines).for_each(|s| {
            s.clear();
            s.reserve(res);
        });
        // fill uninitialized rows within size
        let uninit = lines.saturating_sub(self.buf.len());
        self.buf.reserve(uninit);
        self.buf.extend(std::iter::repeat_with(|| AnsiRow::with_capacity(res)).take(uninit));
    }
    /// Write rows of an image as ANSI colors and half block characters, 2 rows of image pixels per row of ansi,
    /// assumes buf is already cleared
    #[cfg(feature = "rayon")]
    fn _fill<P: RawPixel, C: AnsiColor>(&mut self, img: ImageView<P>, attrs: &ColorAttributes) {
        let (w, h) = img.dimensions();
        self.buf
            .par_iter_mut()
            .take(self.size.1 as usize)
            .zip(
                ArrayView3::from_shape([h as usize, w as usize, P::Repr::CHANNELS], img.data)
                    .expect(concat!(module_path!(), "AnsiImage::_fill: invalid shape"))
                    .axis_chunks_iter(Axis(0), 2)
                    .into_par_iter(),
            )
            .for_each(|(row_buf, pxs)| {
                let mut pxs = pxs.outer_iter();
                match (pxs.next(), pxs.next()) {
                    (Some(fgs), Some(bgs)) => row_buf.extend_fgs_bgs::<P, C>(fgs, bgs, attrs),
                    (Some(fgs), None) => row_buf.extend_fgs::<P, C>(fgs, attrs),
                    _ => unreachable!("Rows in image does not match height"),
                }
            });
    }
    #[cfg(not(feature = "rayon"))]
    fn _fill<P: Pixel, C: AnsiColor>(&mut self, img: &image::ImageBuffer<P, Vec<u8>>, attrs: &ColorAttributes) {
        let (w, h) = img.dimensions();
        let rows = ArrayView3::from_shape([h as usize, w as usize, P::CHANNELS], img)
            .expect(concat!(module_path!(), "AnsiImage::_fill: invalid shape"));
        let mut rows = rows.outer_iter();
        ::std::iter::repeat_with(move || (rows.next(), rows.next()))
            .take(self.size.1 as usize)
            .zip(self.rows_mut())
            .for_each(|(pxs, row_buf)| match pxs {
                (Some(fgs), Some(bgs)) => row_buf.extend_fgs_bgs::<P, C>(fgs, bgs, attrs),
                (Some(fgs), None) => row_buf.extend_fgs::<P, C>(fgs, attrs),
                _ => unreachable!("Rows in image does not match height"),
            });
    }
    /// Get ansi image dimensions from real image dimensions
    #[inline]
    fn _get_size_from((width, height): (u32, u32)) -> (u16, u16) { (width as u16, ((height / 2) + (height % 2)) as u16) }
}

impl<'a> IntoIterator for &'a AnsiImage {
    type Item = &'a AnsiRow;
    type IntoIter = core::slice::Iter<'a, AnsiRow>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter { self.rows() }
}
