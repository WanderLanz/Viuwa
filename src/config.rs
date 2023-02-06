use clap::{value_parser, Parser};

use super::*;

/// A dimension, either a limit or "fit" or "fill"
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dimension {
    #[default]
    Fit,
    Fill,
    Limit(u16),
}
impl FromStr for Dimension {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fit" => Ok(Self::Fit),
            "fill" => Ok(Self::Fill),
            _ => {
                if let Ok(dim) = s.parse::<i16>() {
                    if dim <= 0 {
                        Ok(Self::Fit)
                    } else {
                        Ok(Self::Limit(dim as u16))
                    }
                } else {
                    Err("invalid dimension, must be 'fit' or 'fill' or an integer limit".to_string())
                }
            }
        }
    }
}
impl<'de> Deserialize<'de> for Dimension {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        enum StrOrInt {
            Str(String),
            Int(i16),
        }
        if let Ok(s) = StrOrInt::deserialize(deserializer) {
            match s {
                StrOrInt::Str(ref s) => s.parse().map_err(de::Error::custom),
                StrOrInt::Int(dim) => Ok(if dim <= 0 { Self::Fit } else { Self::Limit(dim as u16) }),
            }
        } else {
            Err(de::Error::custom("invalid dimension, must be 'fit' or 'fill' or an integer limit"))
        }
    }
}

/// The main viuwa configuration struct that is deserialized from the config file and command line
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Config {
    /// The image to display
    pub image: PathBuf,
    /// The log level
    pub log: LogLevel,
    /// The filter to use
    pub filter: FilterType,
    #[serde(skip)]
    pub color_space: ColorSpace,
    #[serde(skip)]
    pub color_depth: ColorDepth,
    /// The color to use
    pub color: ColorType,
    /// Whether to display the image inline
    pub inline: bool,
    /// Whether to clear the screen after displaying the image inline
    pub clear: bool,
    /// The default number of columns to use if the terminal width is unknown
    pub default_columns: Option<u16>,
    /// The default number of rows to use if the terminal height is unknown
    pub default_rows: Option<u16>,
    /// The width of to display image
    pub width: Dimension,
    /// The height of to display image
    pub height: Dimension,
    /// The luma correction to use
    pub luma_correct: u8,
    /// The keybinds to use
    #[cfg(not(target_os = "wasi"))]
    pub keybinds: BTreeMap<KeyBind, Action>,
    /// The keybinds to use
    #[cfg(target_os = "wasi")]
    pub keybinds: BTreeMap<String, Action>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            image: Default::default(),
            log: Default::default(),
            filter: Default::default(),
            color_space: Default::default(),
            color_depth: Default::default(),
            color: Default::default(),
            inline: false,
            clear: false,
            default_columns: Default::default(),
            default_rows: Default::default(),
            width: Default::default(),
            height: Default::default(),
            luma_correct: 100,
            #[cfg(not(target_os = "wasi"))]
            keybinds: BTreeMap::from([
                (KeyBind(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty())), Action::Quit),
                (KeyBind(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty())), Action::Quit),
                (KeyBind(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty())), Action::Help),
                (KeyBind(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty())), Action::Reload),
                (KeyBind(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty())), Action::Cycle(Cyclic::Filter)),
                (KeyBind(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty())), Action::Cycle(Cyclic::ColorSpace)),
                (KeyBind(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty())), Action::Cycle(Cyclic::ColorDepth)),
                (KeyBind(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty())), Action::Cycle(Cyclic::Color)),
            ]),
            #[cfg(target_os = "wasi")]
            keybinds: BTreeMap::from([
                (String::from(""), Action::Quit),
                (String::from("q"), Action::Quit),
                (String::from("h"), Action::Help),
                (String::from("r"), Action::Reload),
                (String::from("f"), Action::Cycle(Cyclic::Filter)),
                (String::from("s"), Action::Cycle(Cyclic::ColorSpace)),
                (String::from("d"), Action::Cycle(Cyclic::ColorDepth)),
                (String::from("c"), Action::Cycle(Cyclic::Color)),
            ]),
        }
    }
}

/// The default command line arguments to use to override the config file
#[derive(Parser, Debug, Default)]
#[command(
        version = env!("CARGO_PKG_VERSION"),
        author = env!("CARGO_PKG_AUTHORS"),
        about = env!("CARGO_PKG_DESCRIPTION"),
        disable_help_flag = true,
)]
#[command(group(
    clap::ArgGroup::new("only_color_type")
        .args([
            "color_space",
            "color_depth",
        ]).conflicts_with("color_type"),
))]
#[command(group(
    clap::ArgGroup::new("log_level")
        .args([
            "quiet",
            "verbose",
        ]).conflicts_with("log"),
))]
pub struct Args {
    /// Print help information
    #[arg(short = '?', long = "help", action = clap::ArgAction::Help)]
    _help: Option<bool>,

    /// Set the level of verbosity
    #[arg(long, value_enum)]
    #[cfg_attr(feature = "env", arg(env = "VIUWA_LOG"))]
    log: Option<LogLevel>,

    /// Suppress verbosity
    #[arg(short, long, action = clap::ArgAction::Count, value_parser = value_parser!(u8).range(0..=3), conflicts_with = "verbose")]
    quiet: u8,

    /// Raise verbosity
    #[arg(short, long, action = clap::ArgAction::Count, value_parser = value_parser!(u8).range(0..=3), conflicts_with = "quiet")]
    verbose: u8,

    /// Specify the path to config.toml file
    #[arg(long, value_name = "CONFIG", value_hint = clap::ValueHint::FilePath, value_parser = parse_file_path_str)]
    #[cfg_attr(feature = "env", arg(env = "VIUWA_CONFIG"))]
    config: Option<PathBuf>,

    /// The image to display
    #[arg(required = true, value_name = "IMAGE", value_hint = clap::ValueHint::FilePath, value_parser = parse_file_path_str)]
    image: PathBuf,

    /// Set resizing filter
    #[arg(short, long, value_parser = FilterType::from_str)]
    #[cfg_attr(feature = "env", arg(env = "VIUWA_FILTER"))]
    filter: Option<FilterType>,

    /// Set color space
    #[arg(long, value_parser = ColorSpace::from_str)]
    #[cfg_attr(feature = "env", arg(env = "VIUWA_COLOR_SPACE"))]
    color_space: Option<ColorSpace>,

    /// Set color depth
    #[arg(long, value_parser = ColorDepth::from_str)]
    #[cfg_attr(feature = "env", arg(env = "VIUWA_COLOR_DEPTH"))]
    color_depth: Option<ColorDepth>,

    /// Set the final color specification
    #[arg(short, long, value_parser = ColorType::from_str)]
    #[cfg_attr(feature = "env", arg(env = "VIUWA_COLOR"))]
    color: Option<ColorType>,

    /// Display the image inline
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    #[cfg_attr(feature = "env", arg(env = "VIUWA_INLINE"))]
    inline: Option<bool>,

    /// Do not display the image inline
    #[arg(long)]
    no_inline: bool,

    /// Clear the terminal after displaying the image inline
    #[arg(long, action = clap::ArgAction::SetTrue, requires = "inline")]
    #[cfg_attr(feature = "env", arg(env = "VIUWA_CLEAR"))]
    clear: bool,

    /// Set the display width of the image
    #[arg(
        short,
        long,
        value_name = "WIDTH",
        value_parser = Dimension::from_str
    )]
    #[cfg_attr(feature = "env", arg(env = "VIUWA_WIDTH"))]
    width: Option<Dimension>,

    /// Set the display height of the image
    #[arg(
        short,
        long,
        value_name = "HEIGHT",
        value_parser = Dimension::from_str
    )]
    #[cfg_attr(feature = "env", arg(env = "VIUWA_HEIGHT"))]
    height: Option<Dimension>,

    /// Luma correction for 256 color mode
    #[arg(
        short,
        long,
        value_parser = value_parser!(u8).range(0..=100),
    )]
    #[cfg_attr(feature = "env", arg(env = "VIUWA_CORRECT"))]
    luma_correct: Option<u8>,
}

impl Args {
    pub fn new() -> Self {
        let mut cli = Args::parse();
        cli.config = cli.config.or_else(config_path);
        if cli.no_inline {
            cli.inline = Some(false);
        }
        cli
    }
    pub fn try_new() -> Result<Self, clap::Error> {
        let mut cli = Args::try_parse()?;
        cli.config = cli.config.or_else(config_path);
        if cli.no_inline {
            cli.inline = Some(false);
        }
        Ok(cli)
    }
}

impl Config {
    pub fn new() -> Self {
        let args = Args::new();
        if let Some(p) = &args.config {
            match ::std::fs::read_to_string(p) {
                Ok(str) => match ::toml::from_str::<Config>(&str) {
                    Ok(con) => {
                        debug!("Config::new", "config.toml {} parsed: {:#?}", p.display(), con);
                        return con.merge_args(args);
                    }
                    Err(e) => error!("could not parse config file: {}: {}", p.display(), e),
                },
                Err(e) => error!("could not read config file: {}: {}", p.display(), e),
            }
        } else {
            debug!("Config::new", "no config file found, using default config");
        }
        Config::default().merge_args(args)
    }
    pub fn merge_args(mut self, args: Args) -> Self {
        self.image = args.image;
        // merge log level
        if let Some(l) = args.log {
            self.log = l;
        } else {
            self.log = (self.log as u8).saturating_add(args.quiet).saturating_sub(args.verbose).into();
        }
        // merge filter
        if let Some(f) = args.filter {
            self.filter = f;
        }
        // merge color type
        if let Some(t) = args.color {
            self.color = t;
            self.color_space = t.space();
            self.color_depth = t.depth();
        } else {
            if let Some(s) = args.color_space {
                self.color_space = s;
            } else {
                self.color_space = self.color.space();
            }
            if let Some(d) = args.color_depth {
                self.color_depth = d;
            } else {
                self.color_depth = self.color.depth();
            }
            self.color = ColorType::from((self.color_space, self.color_depth));
        }
        // merge inline
        if let Some(i) = args.inline {
            self.inline = i;
        }
        // merge clear
        self.clear = args.clear;
        // merge dimensions
        if let Some(w) = args.width {
            self.width = w;
        }
        if let Some(h) = args.height {
            self.height = h;
        }
        // merge luma correction
        if let Some(l) = args.luma_correct {
            self.luma_correct = l;
        }
        self
    }
}

/// Parse a string as a path to a file.
#[inline]
pub fn parse_file_path_str(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("File does not exist: {}", path.display()))
    }
}

/// Get the path to the config file from the the executable directory and environment variables, in that order.
pub fn config_path() -> Option<PathBuf> {
    use std::env::*;
    // check if config file exists in executable directory
    if let Ok(p) = current_exe() {
        if let Some(p) = p.canonicalize().unwrap_or(p).parent().map(|p| p.join("config.toml")) {
            if p.is_file() {
                return Some(p);
            }
        }
    }
    // check if config file exists in XDG_CONFIG_HOME (Unix)
    if let Ok(p) = var("XDG_CONFIG_HOME").map(|p| PathBuf::from(p).join(PathBuf::from_iter(["viuwa", "config.toml"]))) {
        if p.is_file() {
            return Some(p);
        }
    }
    // check if config file exists in APPDATA (Windows)
    if let Ok(p) = var("APPDATA").map(|p| PathBuf::from(p).join(PathBuf::from_iter(["viuwa", "config.toml"]))) {
        if p.is_file() {
            return Some(p);
        }
    }
    // check if config file exists in HOME
    if let Ok(p) = var("HOME").map(|p| PathBuf::from(p).join(PathBuf::from_iter([".config", "viuwa", "config.toml"]))) {
        if p.is_file() {
            return Some(p);
        }
    }
    None
}
