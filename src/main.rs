//! A "super simple" cli/tui ansi image viewer.
//! Uses the `clap` crate for argument parsing.
//! Uses the `image` crate to load images, and `crossterm` to help display them.
//! Uses things from `ansi_colours` crate to help with ansi 256 conversion.

use clap::Parser;

use image::{self, imageops::FilterType};

use std::path::PathBuf;

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

#[derive(Parser)]
#[command(
        version = env!("CARGO_PKG_VERSION"),
        author = env!("CARGO_PKG_AUTHORS"),
        about = env!("CARGO_PKG_DESCRIPTION"),
)]
pub struct Args {
        #[arg(help = "The path to the image to display in ansi", required = true, value_name = "FILE")]
        image: PathBuf,
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
                short,
                long,
                default_value = "1",
                help = "Color type for output: Truecolor (1), 256 (2), Gray (3), 256Gray (4)",
                env = "VIUWA_COLOR",
                value_parser = parse_color_type
        )]
        color: ColorType,
        #[arg(
                short,
                long,
                default_value = "100",
                help = "Luma correction level for 256 color mode, 0-100, 100 is the highest",
                env = "VIUWA_CORRECT",
                value_parser = clap::value_parser!(u32).range(0..=100)
        )]
        luma_correct: u32,
        #[arg(short, long, help = "Do not print warnings or messages", env = "VIUWA_QUIET")]
        quiet: bool,
        #[arg(
                short,
                long,
                help = "Display the ansi image within current line. Useful for dumb terminals and piping to other programs or files",
                env = "VIUWA_INLINE"
        )]
        inline: bool,
        #[arg(
                help = "The width of the ANSI image when inline flag is set",
                short,
                long,
                env = "VIUWA_WIDTH",
                value_name = "WIDTH"
        )]
        width: Option<u16>,
        #[arg(
                help = "The height of the ANSI image when inline flag is set",
                short,
                long,
                env = "VIUWA_HEIGHT",
                value_name = "HEIGHT"
        )]
        height: Option<u16>,
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
                "truecolor" | "1" => Ok(viuwa::ColorType::Color),
                "256" | "2" => Ok(viuwa::ColorType::Color256),
                "gray" | "3" => Ok(viuwa::ColorType::Gray),
                "256gray" | "4" => Ok(viuwa::ColorType::Gray256),
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
                Viuwa::new(orig, args)?.spawn()?;
                Ok(())
        } else {
                Viuwa::inline(orig, args)
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
