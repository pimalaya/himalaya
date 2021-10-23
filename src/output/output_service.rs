use anyhow::{anyhow, Error, Result};
use atty::Stream;
use log::debug;
use serde::Serialize;
use std::{
    convert::{TryFrom, TryInto},
    fmt,
};
use termcolor::{ColorChoice, StandardStream};

use crate::output::Print;

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
            Some(slice) if slice.eq_ignore_ascii_case("json") => Ok(Self::Json),
            Some(slice) if slice.eq_ignore_ascii_case("plain") => Ok(Self::Plain),
            None => Ok(Self::Plain),
            Some(slice) => Err(anyhow!("cannot parse output `{}`", slice)),
        }
    }
}

impl fmt::Display for OutputFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let slice = match self {
            &OutputFmt::Json => "JSON",
            &OutputFmt::Plain => "PLAIN",
        };
        write!(f, "{}", slice)
    }
}

// JSON output helper
/// A little struct-wrapper to provide a JSON output.
#[derive(Debug, Serialize, Clone)]
pub struct OutputJson<T: Serialize> {
    response: T,
}

impl<T: Serialize> OutputJson<T> {
    pub fn new(response: T) -> Self {
        Self { response }
    }
}

pub trait OutputServiceInterface {
    fn print<T: Serialize + Print>(&self, data: T) -> Result<()>;
    fn is_json(&self) -> bool;
}

#[derive(Debug)]
pub struct OutputService {
    fmt: OutputFmt,
}

impl OutputServiceInterface for OutputService {
    /// Print the provided item out according to the formatting setting when you created this
    /// struct.
    fn print<T: Serialize + Print>(&self, data: T) -> Result<()> {
        match self.fmt {
            OutputFmt::Plain => {
                data.print(&mut StandardStream::stdout(if atty::isnt(Stream::Stdin) {
                    // Colors should be deactivated if the terminal is not a tty.
                    ColorChoice::Never
                } else {
                    // Otherwise let's `termcolor` decide by inspecting the environment. From the [doc]:
                    // - If `NO_COLOR` is set to any value, then colors will be suppressed.
                    // - If `TERM` is set to dumb, then colors will be suppressed.
                    // - In non-Windows environments, if `TERM` is not set, then colors will be suppressed.
                    //
                    // [doc]: https://github.com/BurntSushi/termcolor#automatic-color-selection
                    ColorChoice::Auto
                }))?;
            }
            OutputFmt::Json => {
                print!("{}", serde_json::to_string(&OutputJson::new(data))?)
            }
        };
        Ok(())
    }

    /// Returns true, if the formatting should be json.
    fn is_json(&self) -> bool {
        self.fmt == OutputFmt::Json
    }
}

impl Default for OutputService {
    fn default() -> Self {
        Self {
            fmt: OutputFmt::Plain,
        }
    }
}

impl From<&str> for OutputService {
    fn from(fmt: &str) -> Self {
        debug!("init output service");
        debug!("output: `{:?}`", fmt);
        let fmt = fmt.into();
        Self { fmt }
    }
}

impl TryFrom<Option<&str>> for OutputService {
    type Error = Error;

    fn try_from(fmt: Option<&str>) -> Result<Self, Self::Error> {
        debug!("init output service");
        debug!("output: `{:?}`", fmt);
        let fmt = fmt.try_into()?;
        Ok(Self { fmt })
    }
}
