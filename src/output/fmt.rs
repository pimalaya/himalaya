use serde::Serialize;
use std::fmt;

// Output format

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
        write!(
            f,
            "{}",
            match *self {
                OutputFmt::Json => "JSON",
                OutputFmt::Plain => "PLAIN",
            },
        )
    }
}

// Response helper

#[derive(Serialize)]
pub struct Response<T: Serialize + fmt::Display> {
    response: T,
}

impl<T: Serialize + fmt::Display> Response<T> {
    pub fn new(response: T) -> Self {
        Self { response }
    }
}

// Print helper
