use clap::ValueEnum;
use color_eyre::{eyre::eyre, eyre::Error, Result};
use serde::Serialize;
use std::{fmt, str::FromStr};

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
