use error_chain::error_chain;
use std::{
    env,
    fs::{remove_file, File},
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
    remove_file(&draft_path)
        .chain_err(|| format!("Cannot remove file `{}`", draft_path.to_string_lossy()))?;

    Ok(draft)
}

pub fn ask_for_confirmation(prompt: &str) -> Result<()> {
    print!("{} (y/n) ", prompt);
    io::stdout().flush().chain_err(|| "Cannot flush stdout")?;

    match io::stdin()
        .bytes()
        .next()
        .and_then(|res| res.ok())
        .map(|bytes| bytes as char)
    {
        Some('y') | Some('Y') => Ok(()),
        Some(choice) => Err(format!("Invalid choice `{}`", choice).into()),
        None => Err("Empty choice".into()),
    }
}
