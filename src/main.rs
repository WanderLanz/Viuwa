//! A "super simple" cli/tui ansi image viewer.
//! Uses the `clap` crate for argument parsing
//! and the `image` crate to load images.
//! On Unix or Windows systems, the `crossterm` crate is used to help manipulate the terminal
//! and the `rayon` crate is used to parallelize and speed up the conversions.
//! Uses things from `ansi_colours` crate to help with ansi 256 conversion.

// #![feature(get_mut_unchecked)]

use clap::Parser;

use image::{self, imageops::FilterType};

use std::path::PathBuf;

mod errors;

mod viuwa;
use viuwa::*;

pub type BoxResult<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

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
        disable_help_flag = true,
)]
pub struct Args {
        #[arg(
                help = "The path to the image to display in ansi",
                required = true,
                value_name = "FILE"
        )]
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
        #[arg(
                short,
                long,
                help = "Do not print warnings or messages",
                env = "VIUWA_QUIET"
        )]
        quiet: bool,
        #[arg(short = 'H', long = "help", help = "Prints help information", action = clap::ArgAction::Help)]
        help: Option<bool>,
}

fn parse_filter_type(filter: &str) -> Result<FilterType, String> {
        match filter.to_ascii_lowercase().as_str() {
                "nearest" | "1" => Ok(FilterType::Nearest),
                "triangle" | "2" => Ok(FilterType::Triangle),
                "catmullrom" | "3" => Ok(FilterType::CatmullRom),
                "gaussian" | "4" => Ok(FilterType::Gaussian),
                "lanczos3" | "5" => Ok(FilterType::Lanczos3),
                _ => Err("Invalid filter type".into()),
        }
}

fn parse_color_type(format: &str) -> Result<viuwa::ColorType, String> {
        match format.to_ascii_lowercase().as_str() {
                "truecolor" | "1" => Ok(viuwa::ColorType::Color),
                "256" | "2" => Ok(viuwa::ColorType::Color256),
                "gray" | "3" => Ok(viuwa::ColorType::Gray),
                "256gray" | "4" => Ok(viuwa::ColorType::Gray256),
                _ => Err("Invalid color type".into()),
        }
}

/// Very basic check to see if terminal supports ansi
#[cfg(not(windows))]
fn supports_ansi() -> bool { std::env::var("TERM").map_or(false, |term| term != "dumb") }
/// Very basic check to see if terminal supports ansi, and enables Virtual Terminal Processing on Windows
#[cfg(windows)]
fn supports_ansi() -> bool { crossterm::ansi_support::supports_ansi() }
/// Does not work if wasm runtime is restricted
#[cfg(target_family = "wasm")]
fn is_windows() -> bool {
        if let Ok(exe) = std::env::current_exe() {
                exe.canonicalize()
                        .unwrap_or(exe)
                        .extension()
                        .map_or(false, |ext| ext.to_str() == Some("exe"))
        } else {
                std::env::var("OS").map_or_else(
                        |_| {
                                std::env::var("SystemRoot").map_or(false, |s| {
                                        s.to_ascii_lowercase().contains("windows")
                                })
                        },
                        |os| os.to_ascii_lowercase().contains("windows"),
                )
        }
}
#[cfg(target_family = "wasm")]
fn check_warnings() -> Result<(), ()> {
        let is_ansi = supports_ansi();
        let is_win = is_windows();
        if is_win {
                eprintln!("WARNING: Windows support with wasm is unstable, use the native binary instead");
        }
        if !is_ansi {
                eprint!("WARNING: Could not verify that terminal supports ansi. Continue? [Y/n] ");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                let input = input.trim().to_ascii_lowercase();
                if input.starts_with("n") {
                        return Err(());
                }
        }
        Ok(())
}
#[cfg(any(unix, windows))]
fn check_warnings() -> Result<(), ()> {
        let is_ansi = supports_ansi();
        if !is_ansi {
                eprint!("WARNING: Could not verify that terminal supports ansi. Continue? [Y/n] ");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                let input = input.trim().to_ascii_lowercase();
                if input.starts_with("n") {
                        return Err(());
                }
        }
        Ok(())
}

fn main() -> BoxResult<()> {
        let args = Args::parse();
        if let Err(_) = check_warnings() {
                return Ok(());
        };
        let orig = image::open(&args.image)?;
        if !args.inline {
                Viuwa::new(orig, args)?.spawn()?;
                Ok(())
        } else {
                viuwa::inline(orig, args)
        }
}
