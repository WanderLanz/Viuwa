//! A "super simple" cli/tui ansi image viewer.

use clap::Parser;

use image::{self, GenericImageView};

use std::path::PathBuf;

mod errors;

mod viuwa;
use viuwa::resizer::FilterType;
use viuwa::*;

pub type BoxResult<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

/// A threshold for warning the user that the image is too large (width * height).
/// This is a heuristic, and is not guaranteed to be accurate.
const IMAGE_SIZE_THRESHOLD: u32 = 3840 * 2160; // 4k
/// A reasonable maximum width for the terminal.
/// There *should* be noone using a terminal with a width of 1000+ characters... what a horrifying experience.
const MAX_COLS: u16 = 8192;
/// A reasonable maximum height for the terminal.
/// There *should* be noone using a terminal with a height of 1000+ characters... what a horrifying experience.
const MAX_ROWS: u16 = 4096;
/// A reasonable default width for the terminal. This is used when the terminal width cannot be determined.
#[cfg(target_family = "wasm")]
const DEFAULT_COLS: u16 = 80;
/// A reasonable default height for the terminal. This is used when the terminal height cannot be determined.
#[cfg(target_family = "wasm")]
const DEFAULT_ROWS: u16 = 24;
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
    #[arg(short, long, help = "Do not print warnings or messages", env = "VIUWA_QUIET")]
    quiet: bool,
    #[arg(short = 'H', long = "help", help = "Prints help information", action = clap::ArgAction::Help)]
    help: Option<bool>,
}

fn parse_filter_type(filter: &str) -> Result<FilterType, String> {
    use FilterType::*;
    match filter.to_ascii_lowercase().as_str() {
        "nearest" | "1" => Ok(Nearest),
        "triangle" | "2" => Ok(Triangle),
        "catmullrom" | "3" => Ok(CatmullRom),
        "gaussian" | "4" => Ok(Gaussian),
        "lanczos3" | "5" => Ok(Lanczos3),
        _ => Err("Invalid filter type".into()),
    }
}

fn parse_color_type(format: &str) -> Result<viuwa::ColorType, String> {
    use ColorType::*;
    match format.to_ascii_lowercase().as_str() {
        "truecolor" | "1" => Ok(Color),
        "256" | "2" => Ok(Color256),
        "gray" | "3" => Ok(Gray),
        "256gray" | "4" => Ok(Gray256),
        _ => Err("Invalid color type".into()),
    }
}

/// Very basic check to see if terminal supports ansi
#[cfg(not(windows))]
fn supports_ansi() -> bool { std::env::var("TERM").map_or(false, |term| term != "dumb") }
/// Very basic check to see if terminal supports ansi, and enables Virtual Terminal Processing on Windows
#[cfg(windows)]
fn supports_ansi() -> bool { crossterm::ansi_support::supports_ansi() }
/// Does not work if wasm runtime is restricted (most runtimes are)
#[cfg(target_family = "wasm")]
fn is_windows() -> bool {
    if let Ok(exe) = std::env::current_exe() {
        exe.canonicalize()
            .unwrap_or(exe)
            .extension()
            .map_or(false, |ext| ext.to_str() == Some("exe"))
    } else {
        std::env::var("OS").map_or_else(
            |_| std::env::var("SystemRoot").map_or(false, |s| s.to_ascii_lowercase().contains("windows")),
            |os| os.to_ascii_lowercase().contains("windows"),
        )
    }
}
/// Warnings for ansi support and windows
#[cfg(target_family = "wasm")]
fn warnings() -> Result<(), ()> {
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
/// Warnings for ansi support and windows
#[cfg(any(unix, windows))]
fn warnings() -> Result<(), ()> {
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
    if let Err(_) = warnings() {
        return Ok(());
    };
    let orig = image::open(&args.image)?;
    let osize = orig.dimensions();
    // if the image is larger than or equal to 4k, warn the user
    if !args.quiet && (osize.0 * osize.1 >= IMAGE_SIZE_THRESHOLD) {
        eprintln!("WARNING: Image is very large, to avoid performance issues, consider resizing it");
    }
    if !args.inline {
        Viuwa::new(orig, args)?.spawn()
    } else {
        viuwa::inline(orig, args)
    }
}
