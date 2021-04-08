use std::fmt;

pub enum OutputFmt {
    Json,
    Plain,
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
