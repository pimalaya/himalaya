//! # mbox client
//!
//! The wrapper around io-mbox's client every mbox-specific subcommand
//! receives, plus the index sync both the subcommands and the unified
//! adapter go through.
//!
//! The dispatch layer builds it up front and hands it down, the merged
//! [`Account`] riding along as a sibling argument.

use std::{
    fs::{self, File},
    io::{self, Read},
    ops::{Deref, DerefMut},
    path::{Component, Path, PathBuf},
};

use anyhow::{Result, anyhow, bail};
use io_mbox::{
    client::{MboxClient as Inner, MboxClientError},
    coroutine::{MboxCoroutine, MboxYield},
    format::MboxFormat,
    from_line::MboxFromLine,
    index::sync::{MboxIndexSync, MboxIndexSyncOptions, MboxIndexSyncOutcome},
    lock::MboxLockOptions,
    path::{MboxFsPath, MboxPath},
    store::{INBOX, SBD},
};
use pimalaya_cli::error::CodedError;

use crate::{
    account::context::Account,
    config::{
        AccountConfig, Config,
        mbox::{MboxConfig, MboxFormatConfig},
    },
    error::{ErrorCode, mailbox_not_found},
    mbox::cache::MboxCache,
};

/// How many bytes of a file tell whether it opens an mbox.
const HEAD_LEN: usize = 1024;

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
    ///
    /// `INBOX` fails, naming `mbox.inbox`, when the spool is a directory,
    /// or when no spool is set and the root holds no file of that name.
    pub fn resolve_mbox(&self, mailbox: &str) -> Result<MboxFsPath> {
        let path = Path::new(mailbox);
        if path.is_absolute() {
            return Ok(MboxFsPath::from(path));
        }

        let resolved = self.resolve_name(mailbox)?;
        if mailbox.eq_ignore_ascii_case(INBOX) {
            match &self.store.inbox {
                Some(spool) if Path::new(spool.as_str()).is_dir() => {
                    bail!("Mbox spool `{spool}` is a directory: set `mbox.inbox` to the spool file")
                }
                None if !Path::new(resolved.as_str()).exists() => {
                    bail!(
                        "No `mbox.inbox` spool is set, so there is no `{mailbox}`: pass another mailbox, or set `mbox.inbox`"
                    )
                }
                _ => {}
            }
        }

        Ok(resolved)
    }

    /// Resolves a name through the store, which refuses one that is empty
    /// or holds a NUL, a `.` or `..` level, or a `.lock` or `.msf` one.
    pub fn resolve_name(&self, name: &str) -> Result<MboxFsPath> {
        let path = self.store.resolve(&MboxPath::from(name));
        path.ok_or_else(|| MboxClientError::InvalidName(name.to_string()).into())
    }

    /// Resolves a mailbox argument to the mbox file a message command
    /// reads or writes, see [`Self::resolve_mbox`].
    ///
    /// A missing file fails with `mailbox-not-found`; a directory, or a
    /// file that does not open on a `From_` line, is refused before any
    /// lock is taken. A file that cannot be read goes through, for the
    /// command to name the error.
    pub fn open_mbox(&self, mailbox: &str) -> Result<MboxFsPath> {
        let path = self.resolve_mbox(mailbox)?;

        match fs::metadata(path.as_str()) {
            Ok(meta) if meta.is_dir() => {
                bail!("Cannot open `{mailbox}`: it is a directory holding mailboxes, not an mbox")
            }
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Err(mailbox_not_found(format!(
                    "Mbox mailbox `{mailbox}` not found"
                )));
            }
            Err(_) => return Ok(path),
        }

        let mut head = Vec::with_capacity(HEAD_LEN);
        if let Ok(file) = File::open(path.as_str())
            && file.take(HEAD_LEN as u64).read_to_end(&mut head).is_ok()
            && !MboxFromLine::opens_mbox(&head)
        {
            bail!(
                "Cannot open `{mailbox}`: {path} is not an mbox, it does not open on a From_ line"
            );
        }

        Ok(path)
    }

    /// Runs an io-mbox coroutine like [`Inner::run`], its failure coded.
    pub fn exec<C, T, E>(&self, coroutine: C) -> Result<T>
    where
        C: MboxCoroutine<Yield = MboxYield, Return = std::result::Result<T, E>>,
        MboxClientError: From<E>,
    {
        self.run(coroutine).map_err(mbox_error)
    }

    /// Fails when the root is no directory, so a mistyped `mbox.root` does
    /// not list as an empty store.
    pub fn check_root(&self) -> Result<()> {
        let root = self.store.root.as_str();
        if !Path::new(root).is_dir() {
            bail!("Mbox root `{root}` does not exist or is not a directory");
        }
        Ok(())
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
        let output = self.exec(MboxIndexSync::new(path.clone(), index, opts))?;
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
    let account = Account {
        name: Some(name.to_string()),
        ..Account::from(config).merge(Account::from(account_config))
    };
    Ok((account, MboxClient::new(mbox_config)))
}

/// An io-mbox failure as Himalaya reports it: a lock another process
/// holds past `mbox.lock.timeout` carries `store-busy`, and a dotlock the
/// user may not create names the key that skips it.
pub fn mbox_error(err: MboxClientError) -> anyhow::Error {
    if err.lock_busy().is_some() {
        return CodedError::new(
            ErrorCode::StoreBusy,
            format!("{err}; retry once it is released"),
        )
        .into();
    }

    if let MboxClientError::DotlockDenied(path) = &err {
        return anyhow!(
            "Cannot create dotlock {path}: permission denied; set `mbox.lock.dotlock = false` to rely on fcntl locks alone"
        );
    }

    err.into()
}

/// Suffixes of the side files of other tools, which a listing hides.
const SIDE_FILE_SUFFIXES: [&str; 3] = [".lock", ".lck", ".msf"];

/// Rejects an mbox name that is empty, absolute, or contains a `..`
/// component, so a mailbox operation joined to the root cannot escape
/// it, and one a listing would hide: a level starting with `.`, ending in
/// the suffix of a dotlock or a Thunderbird summary, or in the Thunderbird
/// layout (`thunderbird`) in `.sbd`.
pub fn validate_mbox_name(name: &str, thunderbird: bool) -> Result<()> {
    let path = Path::new(name);

    if name.is_empty() {
        bail!("Mbox name must not be empty");
    }

    if path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        bail!("Invalid mbox name `{name}`: it must be relative and must not contain `..`");
    }

    for level in name.split('/') {
        if thunderbird && level.ends_with(SBD) {
            bail!(
                "Invalid mbox name `{name}`: `{level}` ends in `{SBD}`, the folder suffix of the Thunderbird layout, which holds the children of a mailbox"
            );
        }

        let side = SIDE_FILE_SUFFIXES
            .iter()
            .any(|suffix| level.ends_with(suffix));
        if level.starts_with('.') || side {
            bail!(
                "Invalid mbox name `{name}`: `{level}` names a hidden file or the side file of another tool, which no listing shows"
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::config::mbox::MboxLockConfig;

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

        assert_eq!(
            client.resolve_mbox("inbox").unwrap().as_str(),
            "/var/mail/me"
        );
        assert_eq!(
            client.resolve_mbox("a/b").unwrap().as_str(),
            "/mail/a.sbd/b"
        );
        assert_eq!(
            client.resolve_mbox("/tmp/x.mbox").unwrap().as_str(),
            "/tmp/x.mbox"
        );
    }

    #[test]
    fn inbox_without_a_spool_names_the_key() {
        let root = tempfile::tempdir().unwrap();
        let client = MboxClient::new(MboxConfig {
            root: root.path().to_path_buf(),
            inbox: None,
            ..config()
        });

        let err = client.resolve_mbox("INBOX").unwrap_err().to_string();
        assert!(err.contains("`mbox.inbox`"), "{err}");

        std::fs::write(root.path().join("inbox"), b"").unwrap();
        let path = client.resolve_mbox("inbox").unwrap();
        assert_eq!(Path::new(path.as_str()), root.path().join("inbox"));
    }

    #[test]
    fn a_spool_directory_names_the_key() {
        let root = tempfile::tempdir().unwrap();
        let client = MboxClient::new(MboxConfig {
            root: root.path().to_path_buf(),
            inbox: Some(root.path().to_path_buf()),
            ..config()
        });

        let err = client.resolve_mbox("INBOX").unwrap_err().to_string();
        assert!(err.contains("is a directory"), "{err}");
    }

    #[test]
    fn a_busy_lock_is_coded_and_a_denied_dotlock_names_the_key() {
        use io_mbox::{entry::append::MboxEntryAppendError, lock::acquire::MboxLockAcquireError};

        let busy = MboxLockAcquireError::DotlockBusy(MboxFsPath::new("/m/a.lock"));
        let err = mbox_error(MboxClientError::EntryAppend(MboxEntryAppendError::Lock(
            busy,
        )));
        let err = err.context("Add message");
        assert_eq!(
            pimalaya_cli::error::code_of::<ErrorCode>(&err),
            Some(ErrorCode::StoreBusy)
        );

        let err = mbox_error(MboxClientError::DotlockDenied(MboxFsPath::new("/m/a.lock")));
        assert!(
            err.to_string().contains("mbox.lock.dotlock = false"),
            "{err}"
        );
    }

    #[test]
    fn names_stay_under_the_root() {
        assert!(validate_mbox_name("archive", false).is_ok());
        assert!(validate_mbox_name("lists/rust", false).is_ok());
        assert!(validate_mbox_name("", false).is_err());
        assert!(validate_mbox_name("/etc/passwd", false).is_err());
        assert!(validate_mbox_name("a/../../b", false).is_err());
    }

    #[test]
    fn names_a_listing_hides_are_refused() {
        for name in [".hidden", "a/.b", "Y.msf", "Z.lock", "Z.lck"] {
            assert!(validate_mbox_name(name, false).is_err(), "{name}");
        }
        assert!(validate_mbox_name("X.sbd", false).is_ok());
        let err = validate_mbox_name("X.sbd", true).unwrap_err().to_string();
        assert!(
            err.contains("folder suffix of the Thunderbird layout"),
            "{err}"
        );
        assert!(validate_mbox_name("a.sbd/b", true).is_err());
        assert!(validate_mbox_name("a/b", true).is_ok());
    }
}
