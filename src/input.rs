use std::{
    env::temp_dir,
    fmt,
    fs::{remove_file, File},
    io::{self, Read, Write},
    process::Command,
    result,
};

use crate::config::Config;

// Error wrapper

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    AskForSendingConfirmationError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(input): ")?;
        match self {
            Error::IoError(err) => err.fmt(f),
            Error::AskForSendingConfirmationError => write!(f, "action cancelled"),
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

// Utils

fn open_editor_with_tpl(tpl: &[u8]) -> Result<String> {
    // Creates draft file
    let mut draft_path = temp_dir();
    draft_path.push("himalaya-draft.mail");
    File::create(&draft_path)?.write(tpl)?;

    // Opens editor and saves user input to draft file
    Command::new(env!("EDITOR")).arg(&draft_path).status()?;

    // Extracts draft file content
    let mut draft = String::new();
    File::open(&draft_path)?.read_to_string(&mut draft)?;
    remove_file(&draft_path)?;

    Ok(draft)
}

pub fn open_editor_with_new_tpl(config: &Config) -> Result<String> {
    let from = &format!("From: {}", config.email_full());
    let to = "To: ";
    let subject = "Subject: ";
    let headers = [from, to, subject, ""].join("\r\n");

    Ok(open_editor_with_tpl(headers.as_bytes())?)
}

pub fn ask_for_confirmation(prompt: &str) -> Result<()> {
    print!("{} (y/n) ", prompt);
    io::stdout().flush()?;

    match io::stdin()
        .bytes()
        .next()
        .and_then(|res| res.ok())
        .map(|bytes| bytes as char)
    {
        Some('y') | Some('Y') => Ok(()),
        _ => Err(Error::AskForSendingConfirmationError),
    }
}
