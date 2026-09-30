//! # mbox client
//!
//! The wrapper around io-mbox's client every mbox-specific subcommand
//! receives, plus the index sync both the subcommands and the shared
//! adapter go through.
//!
//! The dispatch layer builds it up front and hands it down, the merged
//! [`Account`] riding along as a sibling argument.

use std::{
    ops::{Deref, DerefMut},
    path::{Component, Path, PathBuf},
};

use anyhow::{Result, anyhow, bail};
use io_mbox::{
    client::MboxClient as Inner,
    format::MboxFormat,
    index::sync::{MboxIndexSync, MboxIndexSyncOptions, MboxIndexSyncOutcome},
    lock::MboxLockOptions,
    path::{MboxFsPath, MboxPath},
};

use crate::{
    account::context::Account,
    config::{AccountConfig, Config, MboxConfig, MboxFormatConfig},
    mbox::cache::MboxCache,
};

impl From<MboxFormatConfig> for MboxFormat {
    fn from(format: MboxFormatConfig) -> Self {
        match format {
            MboxFormatConfig::Mboxo => Self::MboxO,
            MboxFormatConfig::Mboxrd => Self::MboxRd,
            MboxFormatConfig::Mboxcl => Self::MboxCl,
            MboxFormatConfig::Mboxcl2 => Self::MboxCl2,
        }
    }
}

/// An mbox client rooted at the configured directory.
pub struct MboxClient {
    inner: Inner,
    /// Where the per-file caches live, `None` when the platform has no
    /// cache directory.
    pub cache_dir: Option<PathBuf>,
}

impl MboxClient {
    /// Builds a client rooted at the configured directory, with the
    /// configured spool, format and locks.
    pub fn new(config: MboxConfig) -> Self {
        let mut inner = Inner::new(config.root.as_path());
        inner.store.inbox = config.inbox.as_deref().map(Into::into);
        inner.store.thunderbird = config.thunderbird;
        inner.format = config.format.into();
        inner.scanner.content_length = inner.format.has_content_length();
        inner.lock = MboxLockOptions {
            skip_dotlock: !config.lock.dotlock,
            skip_fcntl: !config.lock.fcntl,
            timeout_secs: config.lock.timeout,
            stale_secs: None,
        };

        let cache_dir = dirs::cache_dir().map(|dir| dir.join("himalaya").join("mbox"));

        Self { inner, cache_dir }
    }

    /// Resolves a mailbox argument to its file.
    ///
    /// A name goes through the store, `INBOX` reaching the spool. An
    /// absolute path is taken as is, so any mbox file opens without
    /// configuring it, as `mail -f` does.
    pub fn resolve_mbox(&self, mailbox: &str) -> MboxFsPath {
        let path = Path::new(mailbox);
        if path.is_absolute() {
            return MboxFsPath::from(path);
        }
        self.store.resolve(&MboxPath::from(mailbox))
    }

    /// Loads the cache of the mbox at `path` and brings its index up to
    /// date, saving it back when the sync changed anything.
    ///
    /// `rebuild` ignores the cached index, for a read that found it stale.
    pub fn sync(&self, path: &MboxFsPath, rebuild: bool) -> Result<MboxCache> {
        let mut cache = MboxCache::load(self.cache_dir.as_deref(), path);
        let index = if rebuild { None } else { cache.index.take() };

        let opts = MboxIndexSyncOptions {
            scanner: self.scanner.clone(),
            chunk_size: self.chunk_size,
        };
        let output = self.run(MboxIndexSync::new(path.clone(), index, opts))?;
        cache.index = Some(output.index);

        if output.outcome != MboxIndexSyncOutcome::Reused {
            cache.save(self.cache_dir.as_deref(), path);
        }

        Ok(cache)
    }
}

impl Deref for MboxClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MboxClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Opens the mbox client of an already-resolved account, returning it
/// beside the merged [`Account`].
///
/// Bails when the account declares no `[mbox]` block.
pub fn build_mbox_client(
    config: Config,
    name: String,
    mut account_config: AccountConfig,
) -> Result<(Account, MboxClient)> {
    let mbox_config = account_config
        .mbox
        .take()
        .ok_or_else(|| anyhow!("Mbox config is missing for account `{name}`"))?;
    let account = Account::from(config).merge(Account::from(account_config));
    Ok((account, MboxClient::new(mbox_config)))
}

/// Rejects an mbox name that is empty, absolute, or contains a `..`
/// component, so a mailbox operation joined to the root cannot escape
/// it.
pub fn validate_mbox_name(name: &str) -> Result<()> {
    let path = Path::new(name);

    if name.is_empty() {
        bail!("Mbox name must not be empty");
    }

    if path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        bail!("Invalid mbox name `{name}`: it must be relative and must not contain `..`");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::config::MboxLockConfig;

    fn config() -> MboxConfig {
        MboxConfig {
            root: PathBuf::from("/mail"),
            inbox: Some(PathBuf::from("/var/mail/me")),
            format: MboxFormatConfig::Mboxcl2,
            thunderbird: true,
            lock: MboxLockConfig {
                dotlock: false,
                fcntl: true,
                timeout: Some(3),
            },
        }
    }

    #[test]
    fn the_config_reaches_io_mbox() {
        let client = MboxClient::new(config());

        assert_eq!(client.format, MboxFormat::MboxCl2);
        assert!(client.scanner.content_length);
        assert!(client.store.thunderbird);
        assert!(client.lock.skip_dotlock && !client.lock.skip_fcntl);
        assert_eq!(client.lock.timeout_secs, Some(3));
    }

    #[test]
    fn mailboxes_resolve_by_name_or_path() {
        let client = MboxClient::new(config());

        assert_eq!(client.resolve_mbox("inbox").as_str(), "/var/mail/me");
        assert_eq!(client.resolve_mbox("a/b").as_str(), "/mail/a.sbd/b");
        assert_eq!(client.resolve_mbox("/tmp/x.mbox").as_str(), "/tmp/x.mbox");
    }

    #[test]
    fn names_stay_under_the_root() {
        assert!(validate_mbox_name("archive").is_ok());
        assert!(validate_mbox_name("lists/rust").is_ok());
        assert!(validate_mbox_name("").is_err());
        assert!(validate_mbox_name("/etc/passwd").is_err());
        assert!(validate_mbox_name("a/../../b").is_err());
    }
}
