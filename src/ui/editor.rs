use anyhow::{Context, Result};
use log::debug;
use std::{env, fs, process::Command};

use crate::msg::msg_utils;

pub fn open_with_tpl(tpl: String) -> Result<String> {
    let path = msg_utils::local_draft_path();

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

    Ok(content)
}

pub fn open_with_draft() -> Result<String> {
    let path = msg_utils::local_draft_path();
    let tpl =
        fs::read_to_string(&path).context(format!("cannot read local draft at {:?}", path))?;
    open_with_tpl(tpl)
}
