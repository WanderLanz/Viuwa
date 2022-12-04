//! A "super simple" cli/tui ansi image viewer.
use std::{cell::Cell, path::PathBuf};

use clap::{value_parser, CommandFactory, FromArgMatches, Parser, ValueEnum};
use image::{self, GenericImageView};

#[macro_use]
mod macros;
#[macro_use]
mod viuwa;
pub use anyhow::{anyhow, bail, Context, Result};
use viuwa::{resizer::FilterType, *};

#[cfg(feature = "trace")]
mod tracing {
    use core::mem::ManuallyDrop;
    pub struct DropFn<F: FnOnce()>(ManuallyDrop<F>);
    impl<F: FnOnce()> DropFn<F> {
        #[inline]
        pub fn new(f: F) -> Self { Self(ManuallyDrop::new(f)) }
    }
    impl<F: FnOnce()> From<F> for DropFn<F> {
        #[inline]
        fn from(f: F) -> Self { Self::new(f) }
    }
    impl<F: FnOnce()> Drop for DropFn<F> {
        #[inline]
        fn drop(&mut self) { (unsafe { ManuallyDrop::take(&mut self.0) })(); }
    }
}
#[cfg(feature = "trace")]
pub use tracing::*;

/// A threshold for warning the user that the image is too large (width * height).
/// This is a heuristic, and is not guaranteed to be accurate.
const IMAGE_SIZE_THRESHOLD: u32 = 3840 * 2160; // 4k
/// A reasonable maximum width for the terminal.
/// There *should* be noone using a terminal with a width of 1000+ characters?
const MAX_COLS: u16 = 8192;
/// A reasonable maximum height for the terminal.
/// There *should* be noone using a terminal with a height of 1000+ characters?
const MAX_ROWS: u16 = 4096;
/// A reasonable default width for the terminal. This is used when the terminal width cannot be determined.
#[cfg(target_family = "wasm")]
const DEFAULT_COLS: u16 = 80;
/// A reasonable default height for the terminal. This is used when the terminal height cannot be determined.
#[cfg(target_family = "wasm")]
const DEFAULT_ROWS: u16 = 24;
// const LOWER_HALF_BLOCK: &str = "\u{2584}";
const UPPER_HALF_BLOCK: &str = "\u{2580}";
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Silent,
}
impl LogLevel {
    #[inline]
    pub fn enabled(self) -> bool { LOG_LEVEL.with(|cell| cell.get() <= self) }
}
impl From<u8> for LogLevel {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Info,
            1 => Self::Warn,
            2 => Self::Error,
            _ => Self::Silent,
        }
    }
}

thread_local! {
    pub static LOG_LEVEL: Cell<LogLevel> = Cell::new(LogLevel::Info);
}

#[derive(Parser, Debug)]
#[command(
        version = env!("CARGO_PKG_VERSION"),
        author = env!("CARGO_PKG_AUTHORS"),
        about = env!("CARGO_PKG_DESCRIPTION"),
        disable_help_flag = true,
)]
pub struct Args {
    /// Prints help information
    #[arg(short = 'H', long = "help", action = clap::ArgAction::Help)]
    help: Option<bool>,
    /// Prints help information
    #[arg(short = '?', hide = true, action = clap::ArgAction::Help)]
    special_help: Option<bool>,
    /// Level of messaging suppression
    #[arg(short, long, action = clap::ArgAction::Count, value_parser = value_parser!(u8).range(0..=3))]
    quiet: u8,
    /// Manually provide the path to the config.toml file
    #[cfg(feature = "config")]
    #[arg(long, value_name = "FILE", value_hint = clap::ValueHint::FilePath, value_parser = parse_file_path)]
    config: Option<PathBuf>,
    /// Path of the image to display
    #[arg(required = true, value_name = "FILE", value_hint = clap::ValueHint::FilePath, value_parser = parse_file_path)]
    image: PathBuf,
    /// The filter used for resizing the image
    #[arg(short, long, default_value_t = FilterType::Nearest, value_enum, ignore_case = true)]
    filter: FilterType,
    /// The ANSI color format used to display the image
    #[arg(short, long, default_value_t = ColorType::Color, value_enum, ignore_case = true)]
    color: ColorType,
    /// Display the image inline
    #[arg(short, long)]
    inline: bool,
    /// The width of the displayed image
    #[arg(
        short,
        long,
        value_name = "WIDTH",
        requires = "inline",
        value_parser = value_parser!(u16).range(1..MAX_COLS as i64)
    )]
    width: Option<u16>,
    /// The height of the displayed image
    #[arg(
        short,
        long,
        value_name = "HEIGHT",
        requires = "inline",
        value_parser = value_parser!(u16).range(1..MAX_ROWS as i64)
    )]
    height: Option<u16>,
    /// Luma correction for 256 color mode
    #[arg(
        name = "luma-correct",
        short,
        long = "luma-correct",
        default_value = "100",
        value_parser = value_parser!(u32).range(0..=100),
    )]
    luma_correct: u32,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            help: Default::default(),
            special_help: Default::default(),
            quiet: Default::default(),
            #[cfg(feature = "config")]
            config: Default::default(),
            image: Default::default(),
            filter: Default::default(),
            color: Default::default(),
            inline: Default::default(),
            width: Default::default(),
            height: Default::default(),
            luma_correct: 100,
        }
    }
}

#[cfg(feature = "config")]
impl Args {
    /// if an arg is in toml, and the arg in ArgMatches is set by default, then set arg in self to the value in toml
    /// only values with ValueSource::Default are overwritten
    pub fn try_merge_matches_and_toml(
        mut self,
        arg_matches: clap::ArgMatches,
        table: toml::value::Table,
    ) -> (Self, Vec<String>) {
        use clap::parser::ValueSource;
        use toml::value::*;
        let mut errs: Vec<String> = Vec::new();
        macro_rules! err {
            ($l:literal$(,$a:expr)*) => {
                errs.push(format!(concat!($l)$(,$a)*))
            };
        }
        macro_rules! _get {
            (if table.$name:ident is $t:ident then $e:expr) => {
                if let Some($name) = table.get(stringify!($name)) {
                    debug!("Args:try_merge_matches_and_toml", "config contains {}", stringify!($name));
                    if let Value::$t($name) = $name {
                        $e;
                    } else {
                        err!("{} must be {} type", stringify!($name), stringify!($t).to_ascii_lowercase());
                    }
                }
            };
            (if $name:ident.source is $t:ident then $e:expr) => {
                if let Some(ValueSource::$t) = arg_matches.value_source(stringify!($name)) {
                    debug!("Args:try_merge_matches_and_toml", "{}.source={}", stringify!($name), stringify!($t));
                    $e;
                }
            };
            ($name:ident) => {
                _get!(if $name.source is DefaultValue then self.$name = *$name);
            };
            ($name:ident by $p:expr) => {
                _get!(if $name.source is DefaultValue then match $p($name) {
                    Ok($name) => self.$name = $name,
                    Err(e) => err!("{} {}", stringify!($name), e),
                });
            };
        }
        macro_rules! get {
            ($name:ident, $t:ident$(, $p:expr)?) => {
                _get!(if table.$name is $t then _get!($name$( by $p)?));
            };
        }
        #[inline]
        fn enum_from_str<T: ValueEnum>(s: &str) -> Result<T, String> {
            if let Ok(v) = T::from_str(s, true) {
                Ok(v)
            } else {
                Err(format!(
                    "must be one of: {:?}",
                    T::value_variants()
                        .into_iter()
                        .map(|v| v.to_possible_value().unwrap().get_name().to_string())
                        .collect::<Vec<_>>()
                ))
            }
        }
        get!(quiet, Integer, |&v| if v >= 0 && v <= 3 { Ok(v as u8) } else { Err("must be in range [0, 3]".to_string()) });
        get!(filter, String, enum_from_str::<FilterType>);
        get!(color, String, enum_from_str::<ColorType>);
        get!(inline, Boolean);
        get!(width, Integer, |&v| {
            if v > 0 && v < MAX_COLS as i64 {
                Ok(Some(v as u16))
            } else {
                Err(format!("must be between 1 and {}", MAX_COLS))
            }
        });
        get!(height, Integer, |&v| {
            if v > 0 && v < MAX_ROWS as i64 {
                Ok(Some(v as u16))
            } else {
                Err(format!("must be between 1 and {}", MAX_ROWS))
            }
        });
        get!(luma_correct, Integer, |&v| {
            if v >= 0 && v <= 100 {
                Ok(v as u32)
            } else {
                Err(format!("must be between 0 and 100"))
            }
        });
        (self, errs)
    }
}

fn parse_file_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("File does not exist: {}", path.display()))
    }
}
/// Very basic check to see if terminal supports ansi
#[cfg(not(windows))]
fn supports_ansi() -> bool { std::env::var("TERM").map_or(false, |term| term != "dumb") }
/// Very basic check to see if terminal supports ansi, and enables Virtual Terminal Processing on Windows
#[cfg(windows)]
fn supports_ansi() -> bool { crossterm::ansi_support::supports_ansi() }
#[cfg(target_family = "wasm")]
fn is_windows() -> bool {
    std::env::var("OS").map_or_else(
        |_| std::env::var("SystemRoot").map_or(false, |s| s.to_ascii_lowercase().contains("windows")),
        |os| os.to_ascii_lowercase().contains("windows"),
    )
}
/// Warnings for ansi support and windows
#[cfg(target_family = "wasm")]
fn warnings() -> Result<(), ()> {
    let is_ansi = supports_ansi();
    let is_win = is_windows();
    if is_win {
        warn!("Windows support with wasm is unstable, use the native binary instead");
    }
    if !is_ansi {
        warn!("Could not verify that terminal supports ansi. Continue? [Y/n] ");
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
        warn!("Could not verify that terminal supports ansi. Continue? [Y/n] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_ascii_lowercase();
        if input.starts_with("n") {
            return Err(());
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    #[cfg(feature = "debug")]
    {
        eprint!(concat!(fg!(238), "[DEBUG] features: "));
        #[cfg(feature = "rayon")]
        eprint!("rayon,");
        #[cfg(feature = "fir")]
        eprint!("fir,");
        #[cfg(feature = "env")]
        eprint!("env,");
        #[cfg(feature = "config")]
        eprint!("config,");
        #[cfg(feature = "trace")]
        eprint!("trace,");
        eprintln!(concat!("debug", fg!(reset)));
    }
    trace!("main");
    let mut args;
    let matches;
    {
        trace!("main: parsing args");
        let mut cmd = Args::command();
        #[cfg(feature = "env")]
        {
            cmd = cmd
                .mut_arg("quiet", |a| a.env("VIUWA_QUIET"))
                .mut_arg("filter", |a| a.env("VIUWA_FILTER"))
                .mut_arg("color", |a| a.env("VIUWA_COLOR"))
                .mut_arg("inline", |a| a.env("VIUWA_INLINE"))
                .mut_arg("width", |a| a.env("VIUWA_WIDTH"))
                .mut_arg("height", |a| a.env("VIUWA_HEIGHT"))
                .mut_arg("luma-correct", |a| a.env("VIUWA_CORRECT"));
            #[cfg(feature = "config")]
            {
                cmd = cmd.mut_arg("config", |a| a.env("VIUWA_CONFIG"));
            }
        }
        matches = cmd.get_matches();
        args = Args::from_arg_matches(&matches)?;
    }
    debug!("main", "parsed args: {:#?}", args);
    LOG_LEVEL.with(|cell| cell.set(LogLevel::from(args.quiet)));
    #[cfg(feature = "config")]
    'config: {
        trace!("main: config");
        use toml::value::*;
        let config_path = if let Some(config_path) = &args.config {
            config_path.clone()
        } else {
            let config_path;
            #[cfg(not(target_family = "wasm"))]
            {
                let Some(dir) = directories::ProjectDirs::from("","","viuwa") else {
                    info!("Could not find config");
                    break 'config;
                };
                config_path = dir.config_dir().join("config.toml");
            }
            #[cfg(target_family = "wasm")]
            {
                let Ok(config_path_str) = std::env::var("XDG_CONFIG_HOME")
                .map(|base| base + "/viuwa/config.toml")
                .or_else(|_| std::env::var("LOCALAPPDATA").map(|base| base + "/viuwa/config.toml"))
                .or_else(|_| {
                    std::env::var("HOME").map(|base|{
                        base + "/.config/viuwa/config.toml"
                    })
                }) else {
                    warn!("Could not find config folder");
                break 'config;
            };
                config_path = PathBuf::from(config_path_str);
            }
            if !config_path.is_file() {
                info!("Could not find config");
                break 'config;
            }
            config_path
        };
        let Ok(config) = std::fs::read_to_string(&config_path) else {
            error!("Could not read config file at {:?}", &config_path);
            break 'config;
        };
        let Ok(config) = toml::from_str::<Table>(&config) else {
            error!("Could not parse config file at {:?}", &config_path);
            break 'config;
        };
        let errs;
        (args, errs) = args.try_merge_matches_and_toml(matches, config);
        if !errs.is_empty() {
            error!("Failed parsing config file {:?}:", &config_path.to_str().unwrap_or(&config_path.to_string_lossy()));
            for err in errs {
                for line in err.lines() {
                    eprintln!("\t{}", line);
                }
            }
        }
        LOG_LEVEL.with(|cell| cell.set(LogLevel::from(args.quiet)));
        debug!("main", "args with config: {:#?}", args);
    }
    if let Err(_) = warnings() {
        return Ok(());
    };
    let orig = image::open(&args.image).context("Failed to load image, the file extension may be incorrect")?;
    // Any errors from here on out are likely to not be the users fault, so we can ask for a bug report
    human_panic::setup_panic!();

    let osize = orig.dimensions();
    // if the image is larger than or equal to 4k, warn the user
    if osize.0 * osize.1 >= IMAGE_SIZE_THRESHOLD {
        info!("Image is very large, may cause performance issues");
    }
    // unwraps so that we can use panic to report a bug if this fails, (better than opaque errors)
    // most likely due to std::io::stdout() write failing (which *should* *almost* never happen under normal circumstances)
    if !args.inline {
        viuwa::windowed(orig, args).unwrap()
    } else {
        viuwa::inlined(orig, args).unwrap()
    }
    Ok(())
}
