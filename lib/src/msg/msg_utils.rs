use log::{debug, trace};
use std::{env, fs, path};

use crate::msg::{Error, Result};

pub fn local_draft_path() -> path::PathBuf {
    trace!(">> get local draft path");

    let path = env::temp_dir().join("himalaya-draft.eml");
    debug!("local draft path: {:?}", path);

    trace!("<< get local draft path");
    path
}

pub fn remove_local_draft() -> Result<()> {
    trace!(">> remove local draft");

    let path = local_draft_path();
    fs::remove_file(&path).map_err(|err| Error::DeleteLocalDraftError(err, path))?;

    trace!("<< remove local draft");
    Ok(())
}
