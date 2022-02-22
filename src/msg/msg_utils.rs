use anyhow::{Context, Result};
use log::{debug, trace};
use std::{env, fs, path::PathBuf};

pub fn local_draft_path() -> PathBuf {
    let path = env::temp_dir().join("himalaya-draft.mail");
    trace!("local draft path: {:?}", path);
    path
}

pub fn remove_local_draft() -> Result<()> {
    let path = local_draft_path();
    debug!("remove draft path at {:?}", path);
    fs::remove_file(&path).context(format!("cannot remove local draft at {:?}", path))
}
