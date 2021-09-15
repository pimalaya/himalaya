use anyhow::{anyhow, Error, Result};
use log::debug;
use serde::Serialize;
use std::{fmt, str::FromStr};

#[derive(Debug, PartialEq)]
pub enum OutputFmt {
    Plain,
    Json,
}

impl FromStr for OutputFmt {
    type Err = Error;
    fn from_str(slice: &str) -> Result<Self, Self::Err> {
        match slice {
            "JSON" | _ if slice.eq_ignore_ascii_case("json") => Ok(Self::Json),
            "PLAIN" | _ if slice.eq_ignore_ascii_case("plain") => Ok(Self::Plain),
            _ => Err(anyhow!("cannot parse output `{}`", slice)),
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
    fn print<T: Serialize + fmt::Display>(&self, data: T) -> Result<()>;
}

#[derive(Debug)]
pub struct OutputService {
    fmt: OutputFmt,
}

impl OutputService {
    /// Create a new output-handler by setting the given formatting style.
    pub fn new(slice: &str) -> Result<Self> {
        debug!("init output service");
        debug!("output: {}", slice);
        let fmt = OutputFmt::from_str(slice)?;
        Ok(Self { fmt })
    }

    /// Returns true, if the formatting should be plaintext.
    pub fn is_plain(&self) -> bool {
        self.fmt == OutputFmt::Plain
    }

    /// Returns true, if the formatting should be json.
    pub fn is_json(&self) -> bool {
        self.fmt == OutputFmt::Json
    }
}

impl OutputServiceInterface for OutputService {
    /// Print the provided item out according to the formatting setting when you created this
    /// struct.
    fn print<T: Serialize + fmt::Display>(&self, data: T) -> Result<()> {
        match self.fmt {
            OutputFmt::Plain => {
                println!("{}", data)
            }
            OutputFmt::Json => {
                print!("{}", serde_json::to_string(&OutputJson::new(data))?)
            }
        };
        Ok(())
    }
}

impl Default for OutputService {
    fn default() -> Self {
        Self {
            fmt: OutputFmt::Plain,
        }
    }
}
