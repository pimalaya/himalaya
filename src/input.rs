use std::{
    env, fmt,
    fs::{remove_file, File},
    io::{self, Read, Write},
    process::Command,
    result,
};

// Error wrapper

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    GetEditorEnvVarNotFoundError(env::VarError),
    AskForConfirmationDeniedError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "input: ")?;

        match self {
            Error::IoError(err) => err.fmt(f),
            Error::GetEditorEnvVarNotFoundError(err) => err.fmt(f),
            Error::AskForConfirmationDeniedError => write!(f, "action cancelled"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}

impl From<env::VarError> for Error {
    fn from(err: env::VarError) -> Error {
        Error::GetEditorEnvVarNotFoundError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Utils

pub fn open_editor_with_tpl(tpl: &[u8]) -> Result<String> {
    // Creates draft file
    let mut draft_path = env::temp_dir();
    draft_path.push("himalaya-draft.mail");
    File::create(&draft_path)?.write(tpl)?;

    // Opens editor and saves user input to draft file
    Command::new(env::var("EDITOR")?)
        .arg(&draft_path)
        .status()?;

    // Extracts draft file content
    let mut draft = String::new();
    File::open(&draft_path)?.read_to_string(&mut draft)?;
    remove_file(&draft_path)?;

    Ok(draft)
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
        _ => Err(Error::AskForConfirmationDeniedError),
    }
}
