//! Local backend wizard.
//!
//! A typed path pointing at an existing folder configures a local
//! store. When both are compiled in the user picks Maildir or m2dir;
//! otherwise the sole compiled backend is used.

use std::path::PathBuf;

use anyhow::Result;

#[cfg(feature = "m2dir")]
use crate::config::M2dirConfig;
#[cfg(feature = "maildir")]
use crate::config::MaildirConfig;

/// A configured local backend.
pub enum Local {
    #[cfg(feature = "maildir")]
    Maildir(MaildirConfig),
    #[cfg(feature = "m2dir")]
    M2dir(M2dirConfig),
}

/// Configures a local backend rooted at `root`.
pub fn configure(root: PathBuf) -> Result<Local> {
    pick(root)
}

#[cfg(all(feature = "maildir", feature = "m2dir"))]
fn pick(root: PathBuf) -> Result<Local> {
    use pimalaya_cli::prompt;

    const MAILDIR: &str = "Maildir";
    const M2DIR: &str = "m2dir";

    let kind = prompt::item("Local backend:", [MAILDIR, M2DIR], None)?;

    Ok(match kind {
        MAILDIR => Local::Maildir(MaildirConfig { root }),
        _ => Local::M2dir(M2dirConfig { root }),
    })
}

#[cfg(all(feature = "maildir", not(feature = "m2dir")))]
fn pick(root: PathBuf) -> Result<Local> {
    Ok(Local::Maildir(MaildirConfig { root }))
}

#[cfg(all(feature = "m2dir", not(feature = "maildir")))]
fn pick(root: PathBuf) -> Result<Local> {
    Ok(Local::M2dir(M2dirConfig { root }))
}
