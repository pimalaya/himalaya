use std::fmt;

static mut OUTPUT_FMT: &'static OutputFmt = &OutputFmt::Plain;

pub fn set_output_fmt(output_fmt: &'static OutputFmt) {
    unsafe { OUTPUT_FMT = output_fmt }
}

pub unsafe fn get_output_fmt() -> &'static OutputFmt {
    OUTPUT_FMT
}

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
