use anyhow::{anyhow, Error, Result};
use atty::Stream;
use std::{convert::TryFrom, fmt};
use termcolor::ColorChoice;

/// Represents the available output formats.
#[derive(Debug, PartialEq)]
pub enum OutputFmt {
    Plain,
    Json,
}

impl From<&str> for OutputFmt {
    fn from(fmt: &str) -> Self {
        match fmt {
            slice if slice.eq_ignore_ascii_case("json") => Self::Json,
            _ => Self::Plain,
        }
    }
}

impl TryFrom<Option<&str>> for OutputFmt {
    type Error = Error;

    fn try_from(fmt: Option<&str>) -> Result<Self, Self::Error> {
        match fmt {
            Some(fmt) if fmt.eq_ignore_ascii_case("json") => Ok(Self::Json),
            Some(fmt) if fmt.eq_ignore_ascii_case("plain") => Ok(Self::Plain),
            None => Ok(Self::Plain),
            Some(fmt) => Err(anyhow!(r#"cannot parse output format "{}""#, fmt)),
        }
    }
}

impl fmt::Display for OutputFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fmt = match *self {
            OutputFmt::Json => "JSON",
            OutputFmt::Plain => "Plain",
        };
        write!(f, "{}", fmt)
    }
}

/// Defines a struct-wrapper to provide a JSON output.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OutputJson<T: serde::Serialize> {
    response: T,
}

impl<T: serde::Serialize> OutputJson<T> {
    pub fn new(response: T) -> Self {
        Self { response }
    }
}

/// Represent the available color configs.
#[derive(Debug, PartialEq, Eq)]
pub enum ColorFmt {
    Never,
    Always,
    Ansi,
    Auto,
}

impl std::str::FromStr for ColorFmt {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "never" => Ok(Self::Never),
            "always" => Ok(Self::Always),
            "ansi" => Ok(Self::Ansi),
            "auto" => Ok(Self::Auto),
            _ => anyhow::bail!(r#"cannot parse color choice "{}""#, s),
        }
    }
}

impl From<ColorFmt> for ColorChoice {
    fn from(col: ColorFmt) -> Self {
        match col {
            ColorFmt::Never => Self::Never,
            ColorFmt::Always => Self::Always,
            ColorFmt::Ansi => Self::AlwaysAnsi,
            ColorFmt::Auto => {
                if atty::is(Stream::Stdout) {
                    // Otherwise let's `termcolor` decide by inspecting the environment. From the [doc]:
                    // - If `NO_COLOR` is set to any value, then colors will be suppressed.
                    // - If `TERM` is set to dumb, then colors will be suppressed.
                    // - In non-Windows environments, if `TERM` is not set, then colors will be suppressed.
                    //
                    // [doc]: https://github.com/BurntSushi/termcolor#automatic-color-selection
                    Self::Auto
                } else {
                    // Colors should be deactivated if the terminal is not a tty.
                    Self::Never
                }
            }
        }
    }
}
