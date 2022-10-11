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

#[cfg(target_family = "wasm")]
const DEFAULT_COLS: u16 = 80; // default to 80 cols if we can't get the terminal size
#[cfg(target_family = "wasm")]
const DEFAULT_ROWS: u16 = 24; // default to 24 rows if we can't get the terminal size

// const LOWER_HALF_BLOCK: &str = "\u{2584}";
const UPPER_HALF_BLOCK: &str = "\u{2580}";
const COLORTYPE_HELP: &str = "Color type for output: Truecolor (1), 256 (2), Grey (3)";

#[derive(Parser)]
#[command(
        version = env!("CARGO_PKG_VERSION"),
        author = env!("CARGO_PKG_AUTHORS"),
        about = env!("CARGO_PKG_DESCRIPTION"),
)]
struct Args {
        #[arg(help = "The path to the image to display in ansi", required = true, value_name = "FILE")]
        image: PathBuf,
        #[arg(help = "The size of the ANSI image when inline flag is set e.g. \"100x100\". May be necessary for dumb terminals", short, long, env = "VIUWA_SIZE", value_parser = parse_size)]
        size: Option<(u16, u16)>,
        #[arg(
                short = 'f',
                long,
                default_value = "1",
                help = "Filter type for resizing: Nearest (1), Triangle (2), CatmullRom (3), Gaussian (4), Lanczos3 (5)",
                env = "VIUWA_FILTER",
                value_parser = parse_filter_type
        )]
        filter: FilterType,
        #[arg(
                short = 'c',
                long,
                default_value = "1",
                help = COLORTYPE_HELP,
                env = "VIUWA_COLOR",
                value_parser = parse_color_type
        )]
        color: ColorType,
        #[arg(
                short,
                long,
                help = "Display the ansi image within current line. Useful for dumb terminals and piping to other programs or files",
                env = "VIUWA_INLINE"
        )]
        inline: bool,
        #[arg(short, long, help = "Do not print warnings or messages", env = "VIUWA_QUIET")]
        quiet: bool,
}

fn parse_size(size: &str) -> Result<(u16, u16), String> {
        let mut split = size.split('x');
        match (split.next(), split.next()) {
                (Some(w), Some(h)) => {
                        let w = w.parse::<u16>().map_err(|e| e.to_string())?;
                        let h = h.parse::<u16>().map_err(|e| e.to_string())?;
                        if w <= MAX_COLS && h <= MAX_ROWS && w > 0 && h > 0 {
                                Ok((w, h))
                        } else {
                                Err(format!(
                                        "Size should be less than {}x{}, and greater than 0",
                                        MAX_COLS, MAX_ROWS
                                ))
                        }
                }
                _ => Err("Invalid size, use as \"--size [width]x[height]\"".to_string()),
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

fn parse_color_type<'a>(format: &'a str) -> Result<viuwa::ColorType, String> {
        match format.to_ascii_lowercase().as_str() {
                "truecolor" | "1" => Ok(viuwa::ColorType::TrueColor),
                "256" | "2" => Ok(viuwa::ColorType::Ansi256),
                "grey" | "3" => Ok(viuwa::ColorType::AnsiGrey),
                _ => Err("Invalid color type".into()),
        }
}

fn main() -> BoxResult<()> {
        let args = Args::parse();
        if !args.quiet {
                if !supports_ansi() {
                        eprintln!("WARNING: Could not verify that terminal supports ansi");
                }
                #[cfg(target_family = "wasm")]
                if is_windows() {
                        eprintln!("WARNING: Windows support with wasm is unstable, as it may require Win32 API unavailable in wasi");
                }
                eprintln!("Reading image file...");
        }
        // wasi doesn't have universal support for async I/O
        let orig = image::open(&args.image)?;
        if !args.quiet {
                if orig.width() > 1920 && orig.height() > 1080 {
                        eprintln!("WARNING: Large images may cause significant performance issues when changing filter type or resizing");
                }
                eprintln!("Loading ansi...");
        }
        if !args.inline {
                #[cfg(target_family = "wasm")]
                if !args.quiet {
                        eprintln!("WARNING: You may need to press enter to send input to the app");
                }
                Viuwa::new(orig, args.filter, args.color, args.quiet)?.spawn()?;
                Ok(())
        } else {
                Viuwa::inline(orig, args.filter, args.color, args.size, args.quiet)
        }
}

/// Very basic check to see if terminal supports ansi
#[cfg(not(windows))]
fn supports_ansi() -> bool { std::env::var("TERM").map_or(false, |term| term != "dumb") }
/// Very basic check to see if terminal supports ansi, and enables Virtual Terminal Processing on Windows
#[cfg(windows)]
fn supports_ansi() -> bool { crossterm::ansi_support::supports_ansi() }
/// May not work if wasm module does not correctly inherit environment variables
#[cfg(target_family = "wasm")]
fn is_windows() -> bool {
        std::env::var("OS").map_or_else(
                |_| std::env::var("SystemRoot").map_or(false, |s| s.to_ascii_lowercase().contains("windows")),
                |os| os.to_ascii_lowercase().contains("windows"),
        )
}
