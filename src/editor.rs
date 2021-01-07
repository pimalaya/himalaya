use std::env::temp_dir;
use std::fs::{remove_file, File};
use std::io::{self, Read, Write};
use std::process::Command;
use std::{fmt, result};

// Error wrapper

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError(err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Editor utils

fn open_with_template(template: &[u8]) -> Result<String> {
    // Create temporary draft
    let mut draft_path = temp_dir();
    draft_path.push("himalaya-draft.mail");
    File::create(&draft_path)?.write(template)?;

    // Open editor and save user input to draft
    Command::new(env!("EDITOR")).arg(&draft_path).status()?;

    // Read draft
    let mut draft = String::new();
    File::open(&draft_path)?.read_to_string(&mut draft)?;
    remove_file(&draft_path)?;

    Ok(draft)
}

pub fn open_with_new_template() -> Result<String> {
    let template = ["To: ", "Subject: ", ""].join("\r\n");
    open_with_template(template.as_bytes())
}
