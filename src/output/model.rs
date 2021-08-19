use serde::Serialize;
use std::fmt;

// Output format

#[derive(Debug, Eq, PartialEq)]
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

#[derive(Debug, Serialize)]
pub struct OutputJson<T: Serialize> {
    response: T,
}

impl<T: Serialize> OutputJson<T> {
    pub fn new(response: T) -> Self {
        Self { response }
    }
}

// Output

#[derive(Debug)]
pub struct Output {
    fmt: OutputFmt,
}

impl Output {
    pub fn new(fmt: &str) -> Self {
        Self { fmt: fmt.into() }
    }

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

    pub fn is_plain(&self) -> bool {
        self.fmt == OutputFmt::Plain
    }

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
