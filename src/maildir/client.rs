//! Himalaya wrapper around [`io_maildir::client::MaildirClient`].
//!
//! Built up front by the dispatch layer (`crate::cli`) via
//! [`build_maildir_client`] and handed down to every maildir-specific
//! subcommand, together with the merged [`Account`] as a sibling
//! argument.

use std::{
    ops::{Deref, DerefMut},
    path::{Component, Path, PathBuf},
};

use anyhow::{Result, anyhow, bail};
use io_maildir::{client::MaildirClient as Inner, flag::KeywordHeader, maildir::Maildir};

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, KeywordHeaderConfig, MaildirConfig},
};

impl From<KeywordHeaderConfig> for KeywordHeader {
    fn from(header: KeywordHeaderConfig) -> Self {
        match header {
            KeywordHeaderConfig::XKeywords => Self::XKeywords,
            KeywordHeaderConfig::XLabel => Self::XLabel,
        }
    }
}

/// Live Maildir client wrapping io_maildir with the configured root.
pub struct MaildirClient {
    inner: Inner,
    /// Filesystem root of the configured maildir. Kept on the wrapper
    /// so commands can join sub-paths (per-mailbox) without needing
    /// the original [`MaildirConfig`].
    pub root: PathBuf,
}

impl MaildirClient {
    /// Builds a [`MaildirClient`] rooted at the configured maildir
    /// path.
    pub fn new(config: MaildirConfig) -> Self {
        let root = config.root.clone();
        let mut inner = Inner::new(root.to_string_lossy().into_owned());
        inner.dovecot_keywords = config.dovecot_keywords;
        inner.keywords_header = config.keywords_header.map(Into::into);
        Self { inner, root }
    }

    /// Resolves a maildir CLI argument to a loaded [`Maildir`].
    ///
    /// io-maildir resolves every logical name relative to the store root,
    /// so an absolute path — the `id` column of `mailbox list`, or the
    /// configured root itself — is first reduced to its root-relative
    /// name (the empty name, which maps back to the root/INBOX, when the
    /// path *is* the root); a plain relative name (`Archive`,
    /// `Projects/Work`) is loaded as-is. Loading validates the `cur` /
    /// `new` / `tmp` markers.
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

/// Opens the maildir client for an already-resolved account: takes the
/// `[maildir]` block out of `account_config` and builds the merged
/// [`Account`]. Bails when the account has no `[maildir]` block. Returns
/// the client paired with the merged account so subcommands receive both
/// as sibling arguments.
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
