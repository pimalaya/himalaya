//! # Maildir client
//!
//! The wrapper around io-maildir's client every Maildir-specific
//! subcommand receives.
//!
//! The dispatch layer builds it up front and hands it down, the merged
//! [`Account`] riding along as a sibling argument.

use std::{
    ops::{Deref, DerefMut},
    path::{Component, Path, PathBuf},
};

use anyhow::{Result, anyhow, bail};
use io_maildir::{client::MaildirClient as Inner, flag::KeywordHeader, maildir::Maildir};

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, MaildirConfig, MaildirKeywordHeaderConfig},
};

impl From<MaildirKeywordHeaderConfig> for KeywordHeader {
    fn from(header: MaildirKeywordHeaderConfig) -> Self {
        match header {
            MaildirKeywordHeaderConfig::XKeywords => Self::XKeywords,
            MaildirKeywordHeaderConfig::XLabel => Self::XLabel,
        }
    }
}

/// A Maildir client rooted at the configured store.
pub struct MaildirClient {
    inner: Inner,
    /// The filesystem root, kept here so a command joins a sub-path
    /// without reaching back for the configuration.
    pub root: PathBuf,
}

impl MaildirClient {
    /// Builds a client rooted at the configured Maildir path.
    pub fn new(config: MaildirConfig) -> Self {
        let root = config.root.clone();
        let mut inner = Inner::new(root.to_string_lossy().into_owned());
        inner.dovecot_keywords = config.keywords.dovecot;
        inner.keywords_header = config.keywords.header.map(Into::into);
        Self { inner, root }
    }

    /// Resolves a command's Maildir argument into a loaded [`Maildir`].
    ///
    /// io-maildir resolves every logical name relative to the store root,
    /// so an absolute path is first reduced to its root-relative name,
    /// the empty one when it is the root itself. Loading validates the
    /// `cur`, `new` and `tmp` markers.
    pub fn resolve_maildir(&self, path: &Path) -> Result<Maildir> {
        let name = path.strip_prefix(&self.root).unwrap_or(path);
        Ok(self.load_maildir(name.to_string_lossy().into_owned())?)
    }
}

impl Deref for MaildirClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MaildirClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Opens the Maildir client of an already-resolved account, returning it
/// beside the merged [`Account`].
///
/// Bails when the account declares no `[maildir]` block.
pub fn build_maildir_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, MaildirClient)> {
    let maildir_config = account_config
        .maildir
        .take()
        .ok_or_else(|| anyhow!("Maildir config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(account_config));
    Ok((account, MaildirClient::new(maildir_config)))
}

/// Rejects a Maildir folder name that is empty, absolute, or contains a
/// `..` component, so a folder operation joined to the account root
/// cannot escape it.
pub fn validate_maildir_name(name: &Path) -> Result<()> {
    if name.as_os_str().is_empty() {
        bail!("Maildir folder name must not be empty");
    }

    if name.is_absolute() || name.components().any(|c| matches!(c, Component::ParentDir)) {
        bail!(
            "Invalid Maildir folder `{}`: it must be relative and must not contain `..`",
            name.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::validate_maildir_name;

    #[test]
    fn accepts_plain_and_nested_names() {
        assert!(validate_maildir_name(Path::new("Archive")).is_ok());
        assert!(validate_maildir_name(Path::new("Archive/2024")).is_ok());
    }

    #[test]
    fn rejects_empty_absolute_and_parent_dir() {
        assert!(validate_maildir_name(Path::new("")).is_err());
        assert!(validate_maildir_name(Path::new("/etc")).is_err());
        assert!(validate_maildir_name(Path::new("../foo")).is_err());
        assert!(validate_maildir_name(Path::new("a/../../b")).is_err());
    }
}
