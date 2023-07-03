use anyhow::Result;
use dialoguer::Select;
use pimalaya_email::backend::BackendConfig;

use crate::config::wizard::THEME;

#[cfg(feature = "imap-backend")]
use super::imap;
use super::maildir;
#[cfg(feature = "notmuch-backend")]
use super::notmuch;

#[cfg(feature = "imap-backend")]
const IMAP: &str = "IMAP";
const MAILDIR: &str = "Maildir";
#[cfg(feature = "notmuch-backend")]
const NOTMUCH: &str = "Notmuch";
const NONE: &str = "None";

const BACKENDS: &[&str] = &[
    #[cfg(feature = "imap-backend")]
    IMAP,
    MAILDIR,
    #[cfg(feature = "notmuch-backend")]
    NOTMUCH,
    NONE,
];

pub(crate) fn configure(account_name: &str, email: &str) -> Result<BackendConfig> {
    let backend = Select::with_theme(&*THEME)
        .with_prompt("Email backend")
        .items(BACKENDS)
        .default(0)
        .interact_opt()?;

    match backend {
        #[cfg(feature = "imap-backend")]
        Some(idx) if BACKENDS[idx] == IMAP => imap::wizard::configure(account_name, email),
        Some(idx) if BACKENDS[idx] == MAILDIR => maildir::wizard::configure(),
        #[cfg(feature = "notmuch-backend")]
        Some(idx) if BACKENDS[idx] == NOTMUCH => notmuch::wizard::configure(),
        _ => Ok(BackendConfig::None),
    }
}
