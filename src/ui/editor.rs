use anyhow::{anyhow, Context, Result};
use log::{debug, error};
use std::{
    env,
    fs::File,
    io::{Read, Write},
    process::Command,
};

use crate::{
    domain::msg,
    ui::choice::{self, PreEditChoice},
};

pub fn open_editor_with_tpl(tpl: &[u8]) -> Result<String> {
    let path = msg::utils::draft_path();
    if path.exists() {
        debug!("draft found");
        loop {
            match choice::pre_edit() {
                Ok(choice) => match choice {
                    PreEditChoice::Edit => return open_editor_with_draft(),
                    PreEditChoice::Discard => break,
                    PreEditChoice::Quit => return Err(anyhow!("edition aborted")),
                },
                Err(err) => error!("{}", err),
            }
        }
    }

    debug!("create draft");
    File::create(&path)
        .context(format!("cannot create draft file `{:?}`", path))?
        .write(tpl)
        .context(format!("cannot write draft file `{:?}`", path))?;

    debug!("open editor");
    Command::new(env::var("EDITOR").context("cannot find `$EDITOR` env var")?)
        .arg(&path)
        .status()
        .context("cannot launch editor")?;

    debug!("read draft");
    let mut draft = String::new();
    File::open(&path)
        .context(format!("cannot open draft file `{:?}`", path))?
        .read_to_string(&mut draft)
        .context(format!("cannot read draft file `{:?}`", path))?;

    Ok(draft)
}

pub fn open_editor_with_draft() -> Result<String> {
    let path = msg::utils::draft_path();

    // Opens editor and saves user input to draft file
    Command::new(env::var("EDITOR").context("cannot find `EDITOR` env var")?)
        .arg(&path)
        .status()
        .context("cannot launch editor")?;

    // Extracts draft file content
    let mut draft = String::new();
    File::open(&path)
        .context(format!("cannot open file `{:?}`", path))?
        .read_to_string(&mut draft)
        .context(format!("cannot read file `{:?}`", path))?;

    Ok(draft)
}
