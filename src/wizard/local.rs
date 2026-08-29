//! # Local wizard
//!
//! Configures a local store from a typed path pointing at an existing
//! folder.
//!
//! The kind is read off the directory's own markers: an `.m2store` or
//! `.m2dir` marker is m2dir, and a `cur`, `new` and `tmp` tree is
//! Maildir. An empty or ambiguous directory leaves the user to pick, or
//! takes the sole compiled backend when there is one.

use std::path::{Path, PathBuf};

use anyhow::Result;

#[cfg(feature = "m2dir")]
use crate::config::M2dirConfig;
#[cfg(feature = "maildir")]
use crate::config::{MaildirConfig, MaildirKeywordsConfig};

/// A configured local backend.
pub enum Local {
    #[cfg(feature = "maildir")]
    Maildir(MaildirConfig),
    #[cfg(feature = "m2dir")]
    M2dir(M2dirConfig),
}

/// Configures a local backend rooted at `root`, auto-detecting its kind
/// from the on-disk markers and only prompting when that is inconclusive.
pub fn configure(root: PathBuf) -> Result<Local> {
    if let Some(local) = detect(&root) {
        return Ok(local);
    }

    pick(root)
}

/// Detects the backend kind from `root`'s markers: the m2dir `.m2store`
/// (store) or `.m2dir` (single mailbox) marker, or the Maildir
/// `cur`/`new`/`tmp` tree. Returns `None` when no marker of a compiled-in
/// backend is present, leaving the choice to [`pick`].
#[cfg_attr(
    not(all(feature = "maildir", feature = "m2dir")),
    allow(unused_variables)
)]
fn detect(root: &Path) -> Option<Local> {
    #[cfg(feature = "m2dir")]
    if root.join(".m2store").exists() || root.join(".m2dir").exists() {
        return Some(Local::M2dir(M2dirConfig {
            root: root.to_path_buf(),
        }));
    }

    #[cfg(feature = "maildir")]
    if root.join("cur").is_dir() && root.join("new").is_dir() && root.join("tmp").is_dir() {
        return Some(Local::Maildir(MaildirConfig {
            root: root.to_path_buf(),
            keywords: MaildirKeywordsConfig::default(),
        }));
    }

    None
}

#[cfg(all(feature = "maildir", feature = "m2dir"))]
fn pick(root: PathBuf) -> Result<Local> {
    use pimalaya_cli::prompt;

    const MAILDIR: &str = "Maildir";
    const M2DIR: &str = "m2dir";

    let kind = prompt::item("Local backend:", [MAILDIR, M2DIR], None)?;

    Ok(match kind {
        MAILDIR => Local::Maildir(MaildirConfig {
            root,
            keywords: MaildirKeywordsConfig::default(),
        }),
        _ => Local::M2dir(M2dirConfig { root }),
    })
}

#[cfg(all(feature = "maildir", not(feature = "m2dir")))]
fn pick(root: PathBuf) -> Result<Local> {
    Ok(Local::Maildir(MaildirConfig {
        root,
        keywords: MaildirKeywordsConfig::default(),
    }))
}

#[cfg(all(feature = "m2dir", not(feature = "maildir")))]
fn pick(root: PathBuf) -> Result<Local> {
    Ok(Local::M2dir(M2dirConfig { root }))
}
