use anyhow::{anyhow, Error, Result};
use std::{convert::TryFrom, fmt};

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
