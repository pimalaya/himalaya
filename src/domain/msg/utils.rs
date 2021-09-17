use anyhow::{Context, Result};
use log::debug;
use std::{env, fs, path::PathBuf};

pub fn draft_path() -> PathBuf {
    let path = env::temp_dir().join("himalaya-draft.mail");
    debug!("draft path: `{:?}`", path);
    path
}

pub fn remove_draft() -> Result<()> {
    let path = draft_path();
    debug!("remove draft path: `{:?}`", path);
    fs::remove_file(&path).context(format!("cannot delete draft file at `{:?}`", path))
}
