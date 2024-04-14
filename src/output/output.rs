use clap::ValueEnum;
use color_eyre::{eyre::eyre, eyre::Error, Result};
use serde::Serialize;
use std::{
    fmt,
    io::{self, IsTerminal},
    str::FromStr,
};
use termcolor::ColorChoice;

/// Represents the available output formats.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, ValueEnum)]
pub enum OutputFmt {
    #[default]
    Plain,
    Json,
}

impl FromStr for OutputFmt {
    type Err = Error;

    fn from_str(fmt: &str) -> Result<Self, Self::Err> {
        match fmt {
            fmt if fmt.eq_ignore_ascii_case("json") => Ok(Self::Json),
            fmt if fmt.eq_ignore_ascii_case("plain") => Ok(Self::Plain),
            unknown => Err(eyre!("cannot parse output format {}", unknown)),
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OutputJson<T: Serialize> {
    response: T,
}

impl<T: Serialize> OutputJson<T> {
    pub fn new(response: T) -> Self {
        Self { response }
    }
}

/// Represent the available color configs.
#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord, ValueEnum)]
pub enum ColorFmt {
    Never,
    Always,
    Ansi,
    #[default]
    Auto,
}

impl FromStr for ColorFmt {
    type Err = Error;

    fn from_str(fmt: &str) -> Result<Self, Self::Err> {
        match fmt {
            fmt if fmt.eq_ignore_ascii_case("never") => Ok(Self::Never),
            fmt if fmt.eq_ignore_ascii_case("always") => Ok(Self::Always),
            fmt if fmt.eq_ignore_ascii_case("ansi") => Ok(Self::Ansi),
            fmt if fmt.eq_ignore_ascii_case("auto") => Ok(Self::Auto),
            unknown => Err(eyre!("cannot parse color format {}", unknown)),
        }
    }
}

impl From<ColorFmt> for ColorChoice {
    fn from(fmt: ColorFmt) -> Self {
        match fmt {
            ColorFmt::Never => Self::Never,
            ColorFmt::Always => Self::Always,
            ColorFmt::Ansi => Self::AlwaysAnsi,
            ColorFmt::Auto => {
                if io::stdout().is_terminal() {
                    // Otherwise let's `termcolor` decide by
                    // inspecting the environment. From the [doc]:
                    //
                    // * If `NO_COLOR` is set to any value, then
                    // colors will be suppressed.
                    //
                    // * If `TERM` is set to dumb, then colors will be
                    // suppressed.
                    //
                    // * In non-Windows environments, if `TERM` is not
                    // set, then colors will be suppressed.
                    //
                    // [doc]: https://github.com/BurntSushi/termcolor#automatic-color-selection
                    Self::Auto
                } else {
                    // Colors should be deactivated if the terminal is
                    // not a tty.
                    Self::Never
                }
            }
        }
    }
}
