//! # Maildir client
//!
//! The wrapper around io-maildir's client every Maildir-specific
//! subcommand receives.
//!
//! The dispatch layer builds it up front and hands it down, the merged
//! [`Account`] riding along as a sibling argument.

use std::{
    fs,
    ops::{Deref, DerefMut},
    path::{Component, Path, PathBuf},
};

use anyhow::{Result, anyhow, bail};
use io_maildir::{
    client::{MaildirClient as Inner, MaildirClientError},
    flag::KeywordHeader,
    maildir::Maildir,
    path::MaildirFsPath,
};

use crate::{
    account::context::Account,
    config::{
        AccountConfig, Config,
        maildir::{MaildirConfig, MaildirKeywordHeaderConfig, MaildirLayoutConfig},
    },
    email::mailbox::{validate_level, validate_name},
    error::mailbox_not_found,
};

/// The name the Maildir at the store root lists under, the inbox
/// unless a folder of the store already takes that name.
const INBOX: &str = "INBOX";

/// The name the root lists under when a folder takes [`INBOX`]; `-m`
/// resolves it to the root too.
const ROOT: &str = ".";

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
        inner.store.maildirpp = config.layout == MaildirLayoutConfig::MaildirPlusPlus;
        inner.dovecot_keywords = config.keywords.dovecot;
        inner.keywords_header = config.keywords.header.map(Into::into);
        Self { inner, root }
    }

    /// Refuses a root that is not a directory, which io-maildir would list
    /// as an empty store, hiding a typo in `maildir.root`.
    pub fn ensure_root(&self) -> Result<()> {
        if !self.root.is_dir() {
            bail!(
                "Maildir root `{}` does not exist or is not a directory",
                self.root.display()
            );
        }

        Ok(())
    }

    /// Resolves a command's Maildir argument into a loaded [`Maildir`].
    ///
    /// The argument is a name relative to the root, the id `mailbox list`
    /// prints (the absolute path), or the name the root lists under;
    /// see [`Self::logical_name`]. An `inbox` no folder answers to is
    /// the store's inbox, see [`Self::inbox`]. Loading validates the
    /// `cur`, `new` and `tmp` markers.
    pub fn resolve_maildir(&self, mailbox: &Path) -> Result<Maildir> {
        let name = self.logical_name(mailbox)?;

        match self.load_maildir(name.clone()) {
            Ok(maildir) => return Ok(maildir),
            Err(MaildirClientError::NotDir(_)) => {}
            Err(err) => return Err(err.into()),
        }

        let mailbox = mailbox.display();

        if name.eq_ignore_ascii_case("inbox") {
            return self.inbox().ok_or_else(|| {
                mailbox_not_found(format!(
                    "Maildir mailbox `{mailbox}` not found: no folder is named so and the root is no Maildir; set `mailbox.alias.inbox`"
                ))
            });
        }

        Err(mailbox_not_found(format!(
            "Maildir mailbox `{mailbox}` not found"
        )))
    }

    /// The id of the Maildir `mailbox` names: its path on disk, as
    /// `mailbox list` prints it.
    ///
    /// Two arguments naming one Maildir (by name, by path, with a
    /// trailing slash) give one id, so a caller compares them.
    pub fn resolve_mailbox_id(&self, mailbox: &str) -> Result<String> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        Ok(maildir.path().to_string())
    }

    /// The name, relative to the root, of the Maildir argument `mailbox`,
    /// empty for the root itself.
    ///
    /// `.` components and trailing slashes are dropped. An absolute path
    /// must lie under the root, and on Maildir++ an on-disk `.a.b` reads
    /// as `a/b`. A `..` component is refused, so nothing resolves outside
    /// the store.
    pub fn logical_name(&self, mailbox: &Path) -> Result<String> {
        let relative = if mailbox.is_absolute() {
            match mailbox.strip_prefix(&self.root) {
                Ok(relative) => relative,
                Err(_) => bail!(
                    "Maildir mailbox `{}` is outside the store root `{}`",
                    mailbox.display(),
                    self.root.display()
                ),
            }
        } else {
            mailbox
        };

        let mut levels = Vec::new();
        for component in relative.components() {
            match component {
                Component::Normal(level) => levels.push(level.to_string_lossy().into_owned()),
                Component::CurDir => {}
                _ => bail!(
                    "Invalid Maildir mailbox `{}`: it must not contain `..`",
                    mailbox.display()
                ),
            }
        }

        if self.store.maildirpp
            && let [level] = levels.as_slice()
            && level.starts_with('.')
            && level != ROOT
        {
            let on_disk = self.store.root.join(level);
            let name = self.store.relative(&on_disk).unwrap_or_default();
            return Ok(name.to_string());
        }

        Ok(levels.join("/"))
    }

    /// The store's inbox: on the fs layout a top-level folder named
    /// `inbox` in any case, else the root when it is a Maildir; on
    /// Maildir++ the root.
    pub fn inbox(&self) -> Option<Maildir> {
        if !self.store.maildirpp {
            let entries = fs::read_dir(&self.root).ok()?;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.eq_ignore_ascii_case("inbox")
                    && let Ok(maildir) = self.load_maildir(name)
                {
                    return Some(maildir);
                }
            }
        }

        self.load_maildir("").ok()
    }

    /// The name a Maildir is shown and selected by: its path relative
    /// to the store root (`[Gmail]/Archive`), whatever the layout, so
    /// nested Maildirs sharing a last segment stay apart.
    ///
    /// The root lists as `INBOX`, the store's inbox, or as `.` when a
    /// top-level folder named so takes the inbox.
    pub fn mailbox_name(&self, maildir: &Maildir) -> String {
        match self.store.relative(maildir.path()) {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => self.root_name().to_owned(),
        }
    }

    /// Whether `maildir` is the store root.
    pub fn is_root(&self, maildir: &Maildir) -> bool {
        self.store
            .relative(maildir.path())
            .is_some_and(|name| name.is_empty())
    }

    /// The name the root lists under, see [`Self::mailbox_name`].
    fn root_name(&self) -> &'static str {
        let root = MaildirFsPath::from(self.root.as_path());
        match self.inbox() {
            Some(inbox) if inbox.path() != &root => ROOT,
            _ => INBOX,
        }
    }

    /// Rejects a NAME that cannot be one level of the store: see
    /// [`validate_maildir_level`], plus, on Maildir++, a dot, which that
    /// layout reads as the hierarchy delimiter.
    pub fn validate_level(&self, name: &str) -> Result<()> {
        validate_maildir_level(name)?;

        if self.store.maildirpp && name.contains('.') {
            bail!("Maildir++ folder name `{name}` must not hold a dot: it delimits the hierarchy");
        }

        Ok(())
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
    let account = Account {
        name: Some(name.to_string()),
        ..Account::from(config).merge(Account::from(account_config))
    };
    Ok((account, MaildirClient::new(maildir_config)))
}

/// Rejects a Maildir folder name that is empty, absolute, contains a
/// `..` component, or names the root itself (`.`, `./`), so a folder
/// operation joined to the account root cannot escape it nor act on the
/// whole store.
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

    if !name.components().any(|c| matches!(c, Component::Normal(_))) {
        bail!(
            "Invalid Maildir folder `{}`: it names the store root",
            name.display()
        );
    }

    Ok(())
}

/// Rejects a NAME that cannot be one level of a Maildir store: a blank
/// one, one holding a control character or the `/` delimiter, a `cur`,
/// `new` or `tmp` subdirectory name, or one starting with a dot, which
/// the listing reads as hidden.
pub fn validate_maildir_level(name: &str) -> Result<()> {
    validate_name(name)?;
    validate_level(name, '/')?;

    if matches!(name, "cur" | "new" | "tmp") {
        bail!("Maildir folder name `{name}` is reserved: it names a Maildir subdirectory");
    }

    if name.starts_with('.') {
        bail!("Maildir folder name `{name}` must not start with a dot: it would be hidden");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use io_maildir::maildir::Maildir;
    use tempfile::tempdir;

    use super::{MaildirClient, validate_maildir_level, validate_maildir_name};
    use crate::{
        config::maildir::{MaildirConfig, MaildirLayoutConfig},
        error::ErrorCode,
    };

    fn client(root: &Path, layout: MaildirLayoutConfig) -> MaildirClient {
        MaildirClient::new(MaildirConfig {
            root: root.to_path_buf(),
            keywords: Default::default(),
            layout,
        })
    }

    fn maildir_at(path: &Path) {
        for subdir in ["cur", "new", "tmp"] {
            fs::create_dir_all(path.join(subdir)).unwrap();
        }
    }

    #[test]
    fn accepts_plain_and_nested_names() {
        assert!(validate_maildir_name(Path::new("Archive")).is_ok());
        assert!(validate_maildir_name(Path::new("Archive/2024")).is_ok());
    }

    #[test]
    fn rejects_empty_absolute_parent_dir_and_the_root() {
        for name in ["", "/etc", "../foo", "a/../../b", ".", "./", "./."] {
            assert!(validate_maildir_name(Path::new(name)).is_err(), "{name}");
        }
    }

    #[test]
    fn a_level_is_one_listable_folder_name() {
        assert!(validate_maildir_level("Archive").is_ok());
        assert!(validate_maildir_level("Été 2026").is_ok());

        for name in [
            "x/y", "cur", "new", "tmp", ".hidden", "..", ".", "", " ", "a\u{1}b",
        ] {
            assert!(validate_maildir_level(name).is_err(), "{name}");
        }
    }

    #[test]
    fn mailbox_name_is_relative_to_the_root() {
        let client = MaildirClient::new(MaildirConfig {
            root: PathBuf::from("/mail"),
            keywords: Default::default(),
            layout: Default::default(),
        });

        let name = |path: &str| client.mailbox_name(&Maildir::from(path));
        assert_eq!(name("/mail/Archive"), "Archive");
        assert_eq!(name("/mail/[Gmail]/Archive"), "[Gmail]/Archive");
    }

    #[test]
    fn every_spelling_of_a_maildir_resolves_to_one_id() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        maildir_at(root);
        maildir_at(&root.join("Trash"));
        let client = client(root, MaildirLayoutConfig::Fs);

        let trash = root.join("Trash").to_string_lossy().into_owned();
        let spellings = ["Trash", "Trash/", "./Trash", &trash, &format!("{trash}/")];
        for spelling in spellings {
            assert_eq!(
                client.resolve_mailbox_id(spelling).unwrap(),
                trash,
                "{spelling}"
            );
        }

        let root_id = root.to_string_lossy().into_owned();
        for spelling in [".", "", "./", "INBOX", "inbox", &root_id] {
            let id = client.resolve_mailbox_id(spelling).unwrap();
            assert_eq!(id, root_id, "{spelling}");
        }

        assert!(client.resolve_mailbox_id("../x").is_err());
        assert!(client.resolve_mailbox_id("/elsewhere/Trash").is_err());
    }

    #[test]
    fn the_root_lists_as_the_inbox_and_resolves_back() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("store2");
        maildir_at(&root);
        let client = client(&root, MaildirLayoutConfig::Fs);

        let name = client.mailbox_name(&client.resolve_maildir(Path::new(".")).unwrap());
        assert_eq!(name, "INBOX");
        let inbox = client.resolve_maildir(Path::new(&name)).unwrap();
        assert!(client.is_root(&inbox));

        maildir_at(&root.join("Inbox"));
        let root_name = client.mailbox_name(&client.resolve_maildir(Path::new("")).unwrap());
        assert_eq!(root_name, ".");
        let inbox = client.resolve_maildir(Path::new("inbox")).unwrap();
        assert_eq!(client.mailbox_name(&inbox), "Inbox");
    }

    #[test]
    fn a_missing_mailbox_is_coded() {
        let dir = tempdir().unwrap();
        let client = client(dir.path(), MaildirLayoutConfig::Fs);

        let err = client.resolve_maildir(Path::new("Missing")).unwrap_err();
        let code = pimalaya_cli::error::code_of::<ErrorCode>(&err);
        assert_eq!(code, Some(ErrorCode::MailboxNotFound));
        assert_eq!(err.to_string(), "Maildir mailbox `Missing` not found");
    }

    #[test]
    fn a_maildir_plus_plus_store_names_its_folders_by_path() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        maildir_at(root);
        maildir_at(&root.join(".Sent"));
        maildir_at(&root.join(".Lists.rust"));
        let client = client(root, MaildirLayoutConfig::MaildirPlusPlus);

        let mut names: Vec<String> = client
            .list_maildirs()
            .unwrap()
            .iter()
            .map(|maildir| client.mailbox_name(maildir))
            .collect();
        names.sort();
        assert_eq!(names, ["INBOX", "Lists/rust", "Sent"]);

        let rust = root.join(".Lists.rust").to_string_lossy().into_owned();
        assert_eq!(client.resolve_mailbox_id("Lists/rust").unwrap(), rust);
        assert_eq!(client.resolve_mailbox_id(".Lists.rust").unwrap(), rust);
        assert_eq!(client.resolve_mailbox_id(&rust).unwrap(), rust);
        assert!(client.validate_level("a.b").is_err());
    }
}
