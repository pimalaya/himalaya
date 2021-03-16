use error_chain::error_chain;
use std::{
    env,
    fs::File,
    io::{self, Read, Write},
    process::Command,
};

error_chain! {}

pub fn open_editor_with_tpl(tpl: &[u8]) -> Result<String> {
    // Creates draft file
    let mut draft_path = env::temp_dir();
    draft_path.push("himalaya-draft.mail");
    File::create(&draft_path)
        .chain_err(|| format!("Cannot create file `{}`", draft_path.to_string_lossy()))?
        .write(tpl)
        .chain_err(|| format!("Cannot write file `{}`", draft_path.to_string_lossy()))?;

    // Opens editor and saves user input to draft file
    Command::new(env::var("EDITOR").chain_err(|| "Cannot find `EDITOR` env var")?)
        .arg(&draft_path)
        .status()
        .chain_err(|| "Cannot start editor")?;

    // Extracts draft file content
    let mut draft = String::new();
    File::open(&draft_path)
        .chain_err(|| format!("Cannot open file `{}`", draft_path.to_string_lossy()))?
        .read_to_string(&mut draft)
        .chain_err(|| format!("Cannot read file `{}`", draft_path.to_string_lossy()))?;

    Ok(draft)
}

pub fn open_editor_with_draft() -> Result<String> {
    // Creates draft file
    let mut draft_path = env::temp_dir();
    draft_path.push("himalaya-draft.mail");

    // Opens editor and saves user input to draft file
    Command::new(env::var("EDITOR").chain_err(|| "Cannot find `EDITOR` env var")?)
        .arg(&draft_path)
        .status()
        .chain_err(|| "Cannot start editor")?;

    // Extracts draft file content
    let mut draft = String::new();
    File::open(&draft_path)
        .chain_err(|| format!("Cannot open file `{}`", draft_path.to_string_lossy()))?
        .read_to_string(&mut draft)
        .chain_err(|| format!("Cannot read file `{}`", draft_path.to_string_lossy()))?;

    Ok(draft)
}

pub enum Choice {
    Send,
    Draft,
    Edit,
    Quit,
}

pub fn post_edit_choice() -> Result<Choice> {
    print!("(s)end, (d)raft, (e)dit or (q)uit? ");
    io::stdout().flush().chain_err(|| "Cannot flush stdout")?;

    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .chain_err(|| "Cannot read stdin")?;

    match buf.bytes().next().map(|bytes| bytes as char) {
        Some('s') => Ok(Choice::Send),
        Some('d') => Ok(Choice::Draft),
        Some('e') => Ok(Choice::Edit),
        Some('q') => Ok(Choice::Quit),
        Some(choice) => Err(format!("Invalid choice `{}`", choice).into()),
        None => Err("Empty choice".into()),
    }
}
