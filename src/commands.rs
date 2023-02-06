use super::*;

/// Accepted arguments to the cycle command.
/// ```
/// use viuwa::Cyclic;
/// use std::str::FromStr;
/// assert_eq!(Cyclic::from_str("color"), Ok(Cyclic::Color));
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum Cyclic {
    Filter,
    Color,
    ColorDepth,
    ColorSpace,
}
impl FromStr for Cyclic {
    type Err = String;
    #[inline]
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "filter" => Ok(Self::Filter),
            "color" => Ok(Self::Color),
            "color_depth" => Ok(Self::ColorDepth),
            "color_space" => Ok(Self::ColorSpace),
            _ => Err(format!("{s:?} is not a valid Cyclic")),
        }
    }
}
impl<'de> Deserialize<'de> for Cyclic {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
}

/// possible set command key values
/// ```
/// use viuwa::Setting;
/// use viuwa_ansi::ColorDepth;
/// use std::str::FromStr;
/// assert_eq!(Setting::from_str("color_depth 8"), Ok(Setting::ColorDepth(ColorDepth::B8)));
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum Setting {
    Log(LogLevel),
    Filter(FilterType),
    ColorSpace(ColorSpace),
    ColorDepth(ColorDepth),
    Color(ColorType),
    Width(Dimension),
    Height(Dimension),
    LumaCorrect(u8),
}
impl FromStr for Setting {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Assume whitespace is already cleaned
        let mut split = s.splitn(2, |c: char| c.is_ascii_whitespace());
        // Parse the command
        match split.next() {
            Some(s1) => match s1 {
                "log" => Ok(Self::Log(split.next().ok_or(String::from("missing arguments to set log"))?.parse()?)),
                "filter" => Ok(Self::Filter(split.next().ok_or(String::from("missing arguments to set filter"))?.parse()?)),
                "color_space" => {
                    Ok(Self::ColorSpace(split.next().ok_or(String::from("missing arguments to set color_space"))?.parse()?))
                }
                "color_depth" => {
                    Ok(Self::ColorDepth(split.next().ok_or(String::from("missing arguments to set color_depth"))?.parse()?))
                }
                "color" => Ok(Self::Color(split.next().ok_or(String::from("missing arguments to set color"))?.parse()?)),
                "width" => Ok(Self::Width(split.next().ok_or(String::from("missing arguments to set width"))?.parse()?)),
                "height" => Ok(Self::Height(split.next().ok_or(String::from("missing arguments to set height"))?.parse()?)),
                "luma_correct" | "correct" => Ok(Self::LumaCorrect(
                    split
                        .next()
                        .ok_or(String::from("missing arguments to set luma_correct"))?
                        .parse()
                        .map_err(|e| format!("{e}"))?,
                )),
                _ => Err(format!("{s:?} is not a valid SetCommand")),
            },
            None => Err(format!("empty SetCommand")),
        }
    }
}

/// KeyEvent ignoring kind and state
/// ```
/// use viuwa::KeyBind;
/// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
/// use std::str::FromStr;
/// assert_eq!(KeyBind::from_str("ctrl+q"), Ok(KeyBind(KeyEvent::new(KeyCode::Char('q')), KeyModifiers::CONTROL)));
/// ```
#[cfg(not(target_os = "wasi"))]
#[derive(Debug, Clone, Copy, Eq)]
#[repr(transparent)]
pub struct KeyBind(pub KeyEvent);
#[cfg(not(target_os = "wasi"))]
impl FromStr for KeyBind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use KeyCode::*;
        macro_rules! bail {
            () => {
                return Err(format!("{s:?} is not a valid keybind"))
            };
        }
        if s.is_empty() {
            bail!();
        }
        let str = s.to_ascii_lowercase();
        let mut split = str.split_inclusive('+');
        // can't use from_iter because we need to Err
        let mut mods = KeyModifiers::empty();
        let key = split.next_back().unwrap();
        for item in split {
            match &item[..item.len() - 1] {
                "ctrl" => mods.insert(KeyModifiers::CONTROL),
                "alt" => mods.insert(KeyModifiers::ALT),
                "shift" => mods.insert(KeyModifiers::SHIFT),
                // Do we want to read these?
                // "super" => mods.insert(KeyModifiers::SUPER),
                // "meta" => mods.insert(KeyModifiers::META),
                // "hyper" => mods.insert(KeyModifiers::HYPER),
                _ => bail!(),
            }
        }
        Ok(KeyBind(KeyEvent::new(
            if key.len() == 1 {
                let mut c: char = key.chars().next().unwrap();
                if mods.contains(KeyModifiers::SHIFT) {
                    c = c.to_ascii_uppercase();
                }
                Char(c)
            } else if key.starts_with('f') {
                match key[1..].parse::<u8>() {
                    Ok(f) if matches!(f, 1..=24) => F(f),
                    _ => bail!(),
                }
            } else {
                match key {
                    "backspace" => Backspace,
                    "backtab" => {
                        // Crossterm always sends SHIFT with backtab
                        mods.insert(KeyModifiers::SHIFT);
                        BackTab
                    }
                    "del" | "delete" => Delete,
                    "down" => Down,
                    "end" => End,
                    "enter" => Enter,
                    "esc" | "escape" => Esc,
                    "home" => Home,
                    "insert" => Insert,
                    "left" => Left,
                    "pgdn" | "pagedown" => PageDown,
                    "pgup" | "pageup" => PageUp,
                    "plus" => Char('+'),
                    "right" => Right,
                    "space" => Char(' '),
                    "tab" => Tab,
                    "up" => Up,
                    _ => bail!(),
                }
            },
            mods,
        )))
    }
}
#[cfg(not(target_os = "wasi"))]
impl<'de> Deserialize<'de> for KeyBind {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
}
#[cfg(not(target_os = "wasi"))]
impl PartialEq for KeyBind {
    fn eq(&self, Self(KeyEvent { code, modifiers, .. }): &Self) -> bool {
        let Self(KeyEvent { code: c, modifiers: m, .. }) = self;
        (code, modifiers) == (c, m)
    }
}
#[cfg(not(target_os = "wasi"))]
impl PartialOrd for KeyBind {
    fn partial_cmp(&self, Self(KeyEvent { code, modifiers, .. }): &Self) -> Option<std::cmp::Ordering> {
        let Self(KeyEvent { code: c, modifiers: m, .. }) = self;
        (code, modifiers).partial_cmp(&(c, m)).or(Some(std::cmp::Ordering::Equal))
    }
}
#[cfg(not(target_os = "wasi"))]
impl Ord for KeyBind {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.partial_cmp(other).unwrap() }
}
#[cfg(not(target_os = "wasi"))]
impl core::hash::Hash for KeyBind {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let Self(KeyEvent { code, modifiers, .. }) = self;
        (code, modifiers).hash(state)
    }
}

/// The commands available in the pseudo command line.
/// ```
/// use viuwa::Command;
/// assert_eq!(Command::from_str("quit"), Ok(Command::Quit));
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum Command {
    /// Quit the current screen.
    Quit,
    /// Go to the help screen.
    Help,
    /// Redraw.
    Refresh,
    /// Refill the image buffer and redraw.
    Reload,
    /// Set a config value. (e.g. `set log debug`)
    Set(Setting),
    /// Bind a key to a command.
    Bind(
        #[cfg(not(target_os = "wasi"))] KeyBind,
        #[cfg(target_os = "wasi")] String,
        /// The command to bind to be parsed later. (fail quietly if invalid)
        Action,
    ),
    /// Unbind a key
    Unbind(#[cfg(not(target_os = "wasi"))] KeyBind, #[cfg(target_os = "wasi")] String),
    /// Cycle through color or filter modes.
    Cycle(
        /// The mode to cycle.
        Cyclic,
    ),
}
impl FromStr for Command {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Clean whitespace
        let mut in_space = false;
        let s = s
            .trim_matches(|c: char| c.is_ascii_whitespace())
            .chars()
            .filter(move |c| {
                let space = c.is_ascii_whitespace();
                let r = !(in_space && space);
                in_space = space;
                r
            })
            .collect::<String>()
            .to_ascii_lowercase();
        let mut split = s.splitn(2, |c: char| c.is_ascii_whitespace());
        // Parse the command
        match split.next() {
            Some(s1) => match s1 {
                "quit" => Ok(Self::Quit),
                "help" => Ok(Self::Help),
                "refresh" => Ok(Self::Refresh),
                "reload" => Ok(Self::Reload),
                "cycle" => Ok(Self::Cycle(split.next().ok_or(String::from("missing arguments to cycle"))?.parse()?)),
                "unbind" => Ok(Self::Unbind({
                    #[cfg(target_os = "wasi")]
                    {
                        split.next().ok_or(String::from("missing arguments to unbind"))?.to_string()
                    }
                    #[cfg(not(target_os = "wasi"))]
                    {
                        split.next().ok_or(String::from("missing arguments to unbind"))?.parse()?
                    }
                })),
                "set" => Ok(Self::Set(split.next().ok_or(String::from("missing arguments to set"))?.parse()?)),
                "bind" => {
                    let mut split = split
                        .next()
                        .ok_or(String::from("missing arguments to bind"))?
                        .splitn(2, |c: char| c.is_ascii_whitespace());
                    let key = {
                        #[cfg(target_os = "wasi")]
                        {
                            split.next().ok_or(String::from("missing arguments to unbind"))?.to_string()
                        }
                        #[cfg(not(target_os = "wasi"))]
                        {
                            split.next().ok_or(String::from("missing arguments to unbind"))?.parse()?
                        }
                    };
                    Ok(match split.next() {
                        Some(s1) => Self::Bind(key, s1.parse()?),
                        None => Self::Unbind(key),
                    })
                }
                _ => Err(format!("{s:?} is not a valid Command")),
            },
            None => Err(format!("empty Command")),
        }
    }
}
impl<'de> Deserialize<'de> for Command {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
}

/// The commands that a key can be bound to.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum Action {
    Quit,
    Help,
    Refresh,
    Reload,
    Set(Setting),
    Cycle(Cyclic),
}
impl FromStr for Action {
    type Err = String;
    #[inline]
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match Command::from_str(s) {
            Ok(cmd) => match cmd {
                Command::Quit => Ok(Self::Quit),
                Command::Help => Ok(Self::Help),
                Command::Refresh => Ok(Self::Refresh),
                Command::Reload => Ok(Self::Reload),
                Command::Set(setting) => Ok(Self::Set(setting)),
                Command::Cycle(cycle) => Ok(Self::Cycle(cycle)),
                _ => Err(format!("{s:?} cannot be bound to a key")),
            },
            Err(e) => Err(e),
        }
    }
}
impl<'de> Deserialize<'de> for Action {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
}

impl From<Action> for Command {
    #[inline]
    fn from(key_command: Action) -> Self {
        match key_command {
            Action::Quit => Self::Quit,
            Action::Help => Self::Help,
            Action::Refresh => Self::Refresh,
            Action::Reload => Self::Reload,
            Action::Set(setting) => Self::Set(setting),
            Action::Cycle(cycle) => Self::Cycle(cycle),
        }
    }
}
