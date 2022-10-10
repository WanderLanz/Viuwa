//! A "super simple" cli/tui ansi image viewer.
//! Uses the `clap` crate for argument parsing.
//! Uses the `image` crate to load images, and `crossterm` to help display them.

use clap::Parser;

use image::{self, imageops::FilterType};
use std::{iter::Iterator, path::PathBuf};
pub type BoxResult<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;
mod viuwa;
use viuwa::*;
const MAX_COLS: u16 = 8192;
const MAX_ROWS: u16 = 4096;
// const LOWER_HALF_BLOCK: &str = "\u{2584}";
const UPPER_HALF_BLOCK: &str = "\u{2580}";
// const MAX_FILTER_STR_LEN: usize = {
//         let ft_strs: [&str; 5] = ["nearest", "triangle", "catmullrom", "gaussian", "lanczos3"];
//         let mut max = ft_strs[0].len();
//         let mut i = 1;
//         while i < ft_strs.len() {
//                 if ft_strs[i].len() > max {
//                         max = ft_strs[i].len();
//                 }
//                 i += 1;
//         }
//         max
// };
// #[cfg(all(feature = "iterm", feature = "sixel"))]
// const OUTFORMAT_HELP: &str = "Format for output, options are: RGB (1), 256 (2), Grey (3), Iterm (4), Sixel (5)";
// #[cfg(all(not(feature = "iterm"), feature = "sixel"))]
// const OUTFORMAT_HELP: &str = "Format for output, options are: RGB (1), 256 (2), Grey (3), Sixel (5)";
// #[cfg(all(feature = "iterm", not(feature = "sixel")))]
// const OUTFORMAT_HELP: &str = "Format for output, options are: RGB (1), 256 (2), Grey (3), Iterm (4)";
// #[cfg(not(any(feature = "iterm", feature = "sixel")))]
const OUTFORMAT_HELP: &str = "Format for output, options are: RGB (1), 256 (2), Grey (3)";

#[derive(Parser)]
#[command(
        version = env!("CARGO_PKG_VERSION"),
        author = env!("CARGO_PKG_AUTHORS"),
        about = env!("CARGO_PKG_DESCRIPTION"),
)]
struct Args {
        #[arg(help = "The image to display", required = true, value_name = "FILE")]
        image: PathBuf,
        #[arg(help = "The size of the image when inline flag is set, e.g. \"100x100\", may be necessary for terminals that only support color ANSI escapes", short, long, env = "VIUWA_SIZE", value_parser = parse_size)]
        size: Option<(u16, u16)>,
        #[arg(
                short,
                long,
                default_value = "1",
                help = "Filter type for resizing: Nearest (1), Triangle (2), CatmullRom (3), Gaussian (4), Lanczos3 (5)",
                env = "VIUWA_FILTER",
                value_parser = parse_filter_type
        )]
        filter: FilterType,
        #[arg(
                short = 'o',
                long,
                default_value = "1",
                help = OUTFORMAT_HELP,
                env = "VIUWA_FORMAT",
                value_parser = parse_format_type
        )]
        format: OutFormat,
        #[arg(short, long, help = "Display the image within current terminal screen")]
        inline: bool,
}

fn parse_size(size: &str) -> Result<(u16, u16), String> {
        let mut split = size.split('x');
        match (split.next(), split.next()) {
                (Some(w), Some(h)) => {
                        let w = w.parse::<u16>().map_err(|e| e.to_string())?;
                        let h = h.parse::<u16>().map_err(|e| e.to_string())?;
                        if w <= MAX_COLS && h <= MAX_ROWS {
                                Ok((w, h))
                        } else {
                                Err(format!("Size must be less than {}x{}", MAX_COLS, MAX_ROWS))
                        }
                }
                _ => Err("Invalid size, use \"[width]x[height]\"".to_string()),
        }
}

fn parse_filter_type<'a>(filter: &'a str) -> Result<FilterType, String> {
        match filter.to_ascii_lowercase().as_str() {
                "nearest" | "1" => Ok(FilterType::Nearest),
                "triangle" | "2" => Ok(FilterType::Triangle),
                "catmullrom" | "3" => Ok(FilterType::CatmullRom),
                "gaussian" | "4" => Ok(FilterType::Gaussian),
                "lanczos3" | "5" => Ok(FilterType::Lanczos3),
                _ => Err("Invalid filter type".into()),
        }
}

fn parse_format_type<'a>(format: &'a str) -> Result<viuwa::OutFormat, String> {
        match format.to_ascii_lowercase().as_str() {
                "rgb" | "1" => Ok(viuwa::OutFormat::AnsiRgb),
                "256" | "2" => Ok(viuwa::OutFormat::Ansi256),
                "grey" | "3" => Ok(viuwa::OutFormat::AnsiGrey),
                // #[cfg(feature = "iterm")]
                // "iterm" | "4" => Ok(viuwa::OutFormat::Iterm),
                // #[cfg(not(feature = "iterm"))]
                // "iterm" | "4" => Err("Iterm feature is not enabled".into()),
                // #[cfg(feature = "sixel")]
                // "sixel" | "5" => Ok(viuwa::OutFormat::Sixel),
                // #[cfg(not(feature = "sixel"))]
                // "sixel" | "5" => Err("Sixel feature is not enabled".into()),
                _ => Err("Invalid format type".into()),
        }
}

fn main() -> BoxResult<()> {
        let args = Args::parse();
        eprintln!("Loading image...");
        let orig = image::open(&args.image)?;
        eprintln!("Starting app...");
        if !args.inline {
                #[cfg(windows)]
                if !::crossterm::ansi_support::supports_ansi() {
                        return Err("detected no ansi support for windows".into());
                }
                #[cfg(not(any(windows, unix)))]
                eprintln!("WARNING: Without the inline flag, you may need to press enter to send input to the app");
                Viuwa::new(orig, args.filter, args.format)?.spawn()?;
                Ok(())
        } else {
                Viuwa::inline(orig, args.filter, args.format, args.size)
        }
}
