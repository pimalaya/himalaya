use anyhow::{Context, Result};
use log::debug;
use std::{env, fs, process::Command};

use crate::domain::msg::{self, Tpl};

pub fn open_with_tpl(tpl: Tpl) -> Result<Tpl> {
    let path = msg::utils::local_draft_path();

    debug!("create draft");
    fs::write(&path, tpl.as_bytes()).context(format!("cannot write local draft at {:?}", path))?;

    debug!("open editor");
    Command::new(env::var("EDITOR").context(r#"cannot find "$EDITOR" env var"#)?)
        .arg(&path)
        .status()
        .context("cannot launch editor")?;

    debug!("read draft");
    let content =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;

    Ok(Tpl(content))
}

pub fn open_with_draft() -> Result<Tpl> {
    let path = msg::utils::local_draft_path();
    let content =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;
    let tpl = Tpl(content);
    open_with_tpl(tpl)
}
