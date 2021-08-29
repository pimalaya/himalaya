use serde::Serialize;
use std::fmt;

// Output format

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum OutputFmt {
    Plain,
    Json,
}

impl From<&str> for OutputFmt {
    fn from(s: &str) -> Self {
        match s {
            "json" => Self::Json,
            "plain" | _ => Self::Plain,
        }
    }
}

impl fmt::Display for OutputFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fmt = match *self {
            OutputFmt::Json => "JSON",
            OutputFmt::Plain => "PLAIN",
        };

        write!(f, "{}", fmt)
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

// Output
/// A simple wrapper for a general formatting.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Output {
    fmt: OutputFmt,
}

impl Output {
    /// Create a new output-handler by setting the given formatting style.
    pub fn new(fmt: &str) -> Self {
        Self { fmt: fmt.into() }
    }

    /// Print the provided item out according to the formatting setting when you created this
    /// struct.
    pub fn print<T: Serialize + fmt::Display>(&self, item: T) {
        match self.fmt {
            OutputFmt::Plain => {
                println!("{}", item)
            }
            OutputFmt::Json => {
                print!("{}", serde_json::to_string(&OutputJson::new(item)).unwrap())
            }
        }
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

impl Default for Output {
    fn default() -> Self {
        Self {
            fmt: OutputFmt::Plain,
        }
    }
}
