use error_chain::error_chain;
use log::{debug, error, trace};
use std::{
    env,
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
    process::Command,
};

error_chain! {
    foreign_links {
        Utf8(std::string::FromUtf8Error);
    }
}

fn draft_path() -> PathBuf {
    env::temp_dir().join("himalaya-draft.mail")
}

pub fn remove_draft() -> Result<()> {
    debug!("[input] remove draft");

    let draft_path = draft_path();
    debug!("[input] draft path: {:?}", draft_path);

    fs::remove_file(&draft_path)
        .chain_err(|| format!("Could not delete draft file {:?}", draft_path))
}

pub fn open_editor_with_tpl(tpl: &[u8]) -> Result<String> {
    debug!("[input] open editor with tpl");
    trace!("{}", String::from_utf8(tpl.to_vec())?);

    let draft_path = draft_path();
    debug!("[input] draft path: {:?}", draft_path);

    if draft_path.exists() {
        debug!("[input] draft found");
        loop {
            match pre_edit_choice() {
                Ok(choice) => match choice {
                    PreEditChoice::Edit => return open_editor_with_draft(),
                    PreEditChoice::Discard => break,
                    PreEditChoice::Quit => return Err("Edition aborted".into()),
                },
                Err(err) => error!("{}", err),
            }
        }
    }

    debug!("[Input] create draft");
    File::create(&draft_path)
        .chain_err(|| format!("Could not create draft file {:?}", draft_path))?
        .write(tpl)
        .chain_err(|| format!("Could not write draft file {:?}", draft_path))?;

    debug!("[Input] open editor");
    Command::new(env::var("EDITOR").chain_err(|| "Could not find `EDITOR` env var")?)
        .arg(&draft_path)
        .status()
        .chain_err(|| "Could not launch editor")?;

    debug!("[Input] read draft");
    let mut draft = String::new();
    File::open(&draft_path)
        .chain_err(|| format!("Could not open draft file {:?}", draft_path))?
        .read_to_string(&mut draft)
        .chain_err(|| format!("Could not read draft file {:?}", draft_path))?;

    Ok(draft)
}

pub fn open_editor_with_draft() -> Result<String> {
    debug!("[input] open editor with draft");

    let draft_path = draft_path();
    debug!("[input] draft path: {:?}", draft_path);

    // Opens editor and saves user input to draft file
    Command::new(env::var("EDITOR").chain_err(|| "Could not find `EDITOR` env var")?)
        .arg(&draft_path)
        .status()
        .chain_err(|| "Could not launch editor")?;

    // Extracts draft file content
    let mut draft = String::new();
    File::open(&draft_path)
        .chain_err(|| format!("Could not open file {:?}", draft_path))?
        .read_to_string(&mut draft)
        .chain_err(|| format!("Could not read file {:?}", draft_path))?;

    Ok(draft)
}

pub enum PreEditChoice {
    Edit,
    Discard,
    Quit,
}

pub fn pre_edit_choice() -> Result<PreEditChoice> {
    debug!("[input] pre edit choice");

    println!("A draft was found:");
    print!("(e)dit, (d)iscard or (q)uit? ");
    io::stdout()
        .flush()
        .chain_err(|| "Could not flush stdout")?;

    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .chain_err(|| "Could not read stdin")?;

    match buf.bytes().next().map(|bytes| bytes as char) {
        Some('e') => {
            debug!("[input] pre edit choice: edit matched");
            Ok(PreEditChoice::Edit)
        }
        Some('d') => {
            debug!("[input] pre edit choice: discard matched");
            Ok(PreEditChoice::Discard)
        }
        Some('q') => {
            debug!("[input] pre edit choice: quit matched");
            Ok(PreEditChoice::Quit)
        }
        Some(choice) => {
            debug!("[input] pre edit choice: invalid choice {}", choice);
            Err(format!("Invalid choice `{}`", choice).into())
        }
        None => {
            debug!("[input] pre edit choice: empty choice");
            Err("Empty choice".into())
        }
    }
}

pub enum PostEditChoice {
    Send,
    Edit,
    LocalDraft,
    RemoteDraft,
    Discard,
}

pub fn post_edit_choice() -> Result<PostEditChoice> {
    print!("(s)end, (e)dit, (l)ocal/(r)emote draft or (d)iscard? ");
    io::stdout()
        .flush()
        .chain_err(|| "Could not flush stdout")?;

    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .chain_err(|| "Could not read stdin")?;

    match buf.bytes().next().map(|bytes| bytes as char) {
        Some('s') => Ok(PostEditChoice::Send),
        Some('l') => Ok(PostEditChoice::LocalDraft),
        Some('r') => Ok(PostEditChoice::RemoteDraft),
        Some('e') => Ok(PostEditChoice::Edit),
        Some('d') => Ok(PostEditChoice::Discard),
        Some(choice) => Err(format!("Invalid choice `{}`", choice).into()),
        None => Err("Empty choice".into()),
    }
}
