use serde::Serialize;
use std::{
    fmt::{self, Display},
    io,
    process::Command,
    result, string,
};

// Error wrapper

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    ParseUtf8Error(string::FromUtf8Error),
    SerializeJsonError(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "input: ")?;

        match self {
            Error::IoError(err) => err.fmt(f),
            Error::ParseUtf8Error(err) => err.fmt(f),
            Error::SerializeJsonError(err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}

impl From<string::FromUtf8Error> for Error {
    fn from(err: string::FromUtf8Error) -> Error {
        Error::ParseUtf8Error(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::SerializeJsonError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Utils

pub fn run_cmd(cmd: &str) -> Result<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", cmd]).output()?
    } else {
        Command::new("sh").arg("-c").arg(cmd).output()?
    };

    Ok(String::from_utf8(output.stdout)?)
}

pub fn print<T: Display + Serialize>(output_type: &str, item: T) -> Result<()> {
    match output_type {
        "json" => print!("{}", serde_json::to_string(&item)?),
        "text" | _ => println!("{}", item.to_string()),
    }

    Ok(())
}
