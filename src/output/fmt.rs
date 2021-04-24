use serde::Serialize;
use std::fmt;

pub static mut OUTPUT_FMT: &'static OutputFmt = &OutputFmt::Plain;

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

#[derive(Serialize)]
pub struct Response<T: Serialize + fmt::Display> {
    response: T,
}

impl<T: Serialize + fmt::Display> Response<T> {
    pub fn new(response: T) -> Self {
        Self { response }
    }
}

impl<T: Serialize + fmt::Display> fmt::Display for Response<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            match get_output_fmt() {
                &OutputFmt::Plain => {
                    writeln!(f, "{}", self.response)
                }
                &OutputFmt::Json => {
                    write!(f, "{}", serde_json::to_string(self).unwrap())
                }
            }
        }
    }
}
