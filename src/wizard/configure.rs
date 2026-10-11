//! # Configure command
//!
//! The `configure` command: the wizard generates an account, it never
//! edits one.
//!
//! An account is discovered from one prompt, tested, then handed back as
//! a file to create, a block to append or a document on stdout. Whatever
//! discovery does not cover is written by hand against the sample.
//!
//! It runs from `himalaya configure`, and from the offer a bare
//! `himalaya` raises when it finds no configuration. That offer is the
//! only place the wizard introduces itself, the command asked for by name
//! going straight to the prompts.
//!
//! Appending is a plain text append rather than a re-serialization, so
//! comments, ordering and hand-written formatting survive. Two rules
//! guard it: the account name has to be free, two tables of one name
//! making the whole document fail to parse, and the new account claims
//! the default only when no other one does.

use std::{
    fmt,
    fs::{self, DirBuilder, File, OpenOptions},
    io::{ErrorKind, IsTerminal, Read, Write, stdin, stdout},
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

use anyhow::{Context, Result, bail};
use clap::Parser;
use pimalaya_cli::{printer::Printer, prompt};
use pimalaya_config::toml::TomlConfig;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    config::{CONFIG_SAMPLE_URL, Config},
    wizard::discover,
};

/// Configure an account interactively.
///
/// Discovers a provider from an email address, a server URL or a local
/// folder path, tests the connection, then writes the account, appends it
/// to the configuration already there, or prints it to be placed by hand.
///
/// Anything discovery does not cover is written by hand.
#[derive(Debug, Parser)]
pub struct ConfigureCommand;

impl ConfigureCommand {
    /// Runs the wizard, then saves, appends or prints the account.
    ///
    /// No welcome here: whoever typed the command knows what it does, the
    /// banner belonging to the offer a missing configuration raises. The
    /// account name is not asked either, being only the table key.
    ///
    /// A redirected stdout and the JSON output both stay
    /// non-interactive, the document going to stdout and no file being
    /// touched. The prompts render on stderr, out of that document.
    pub fn execute(self, printer: &mut impl Printer, config_paths: &[PathBuf]) -> Result<()> {
        if !stdin().is_terminal() {
            bail!(
                "Configuring needs a terminal to prompt on, \
                 write the configuration by hand instead: {CONFIG_SAMPLE_URL}"
            );
        }

        let path = Config::target_path(config_paths)?;
        let existing = ExistingConfig::read(&path)?;

        let (base_name, mut account) = discover::run()?;
        let name = account_name(&base_name, existing.as_ref());

        // NOTE: a second `default = true` would make the account every
        // command picks depend on map ordering, so the generated one
        // claims the default only when no other account does.
        let default = !existing.as_ref().is_some_and(|config| config.has_default);
        account.default = default;

        let config = ConfigureOutput {
            document: account.render(&name)?,
            name,
            default,
        };

        if printer.is_json() || !stdout().is_terminal() {
            return printer.out(config);
        }

        match existing {
            Some(existing) => append_or_print(printer, &path, config, &existing.snapshot),
            None => save_or_print(printer, &path, config),
        }
    }
}

/// What a configuration file already on disk constrains in the generated
/// account: the names it takes, and whether one of its accounts already
/// claims the default.
struct ExistingConfig {
    names: Vec<String>,
    has_default: bool,
    snapshot: ConfigSnapshot,
}

impl ExistingConfig {
    /// Reads the configuration at the given path, `None` when there is no
    /// file there.
    ///
    /// A file that fails to parse errors rather than read as absent:
    /// appending to a broken document would bury the real problem under a
    /// second one.
    fn read(path: &Path) -> Result<Option<Self>> {
        let Some(snapshot) = ConfigSnapshot::read(path)? else {
            return Ok(None);
        };
        let config: Config = toml::from_str(&snapshot.contents)
            .with_context(|| format!("Parse the configuration at {}", path.display()))?;

        Ok(Some(Self {
            names: config.accounts.keys().cloned().collect(),
            has_default: config.accounts.values().any(|account| account.default),
            snapshot,
        }))
    }
}

/// The generated account, as the printer takes it.
#[derive(Serialize, JsonSchema)]
pub struct ConfigureOutput {
    /// The account name, which is the `[accounts.<name>]` table key.
    name: String,
    /// Whether the account claims the default.
    default: bool,
    /// The rendered TOML document.
    document: String,
}

impl fmt::Display for ConfigureOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: the trailing newline terminates the document, and it is
        // also what flushes the line-buffered stdout.
        writeln!(f, "{}", self.document.trim_end())
    }
}

/// Frames Himalaya, names the missing configuration file, and points at
/// the sample for what the wizard does not cover.
///
/// It runs before the offer a bare `himalaya` raises, where the wizard
/// meets someone who did not ask for it, and `configure` skips it. On
/// stderr, so a redirected stdout holds the document alone.
pub fn print_welcome(path: &Path) {
    eprintln!();
    eprintln!("Welcome to Himalaya, the CLI to manage emails.");
    eprintln!();
    eprintln!("Himalaya talks to your existing mailbox over IMAP, JMAP, Gmail, Microsoft");
    eprintln!("Graph or a local Maildir. It needs one account to know which mailbox to");
    eprintln!("read, and no configuration file was found at:");
    eprintln!();
    eprintln!("  {}", path.display());
    eprintln!();
    eprintln!("The wizard discovers a provider's settings from your email address, tests");
    eprintln!("the connection and generates a ready-to-use account. Everything discovery");
    eprintln!("does not cover is written by hand, and every field is documented at:");
    eprintln!();
    eprintln!("  {CONFIG_SAMPLE_URL}");
    eprintln!();
    eprintln!("At anytime, you can create a new account with the command:");
    eprintln!();
    eprintln!("  himalaya configure");
    eprintln!();
}

/// The name discovery proposes, suffixed until the configuration does not
/// hold it already.
///
/// It is never prompted, being only the table key, but it does have to be
/// free: a second table of one name makes the whole document fail to
/// parse, taking the accounts that used to work down with it.
fn account_name(base: &str, existing: Option<&ExistingConfig>) -> String {
    let taken = existing
        .map(|config| config.names.as_slice())
        .unwrap_or(&[]);

    if !taken.iter().any(|name| name == base) {
        return base.to_string();
    }

    let mut suffix = 2;

    loop {
        let name = format!("{base}-{suffix}");

        if !taken.contains(&name) {
            return name;
        }

        suffix += 1;
    }
}

/// Offers to write the generated account to a configuration file that
/// does not exist yet, printing it instead when the offer is declined.
fn save_or_print(printer: &mut impl Printer, path: &Path, config: ConfigureOutput) -> Result<()> {
    let prompt = format!("Save this account to {}?", path.display());

    if !prompt::bool(prompt, true)? {
        return printer.out(config);
    }

    write_configuration(path, &config.to_string(), None)?;

    print_saved(path, &config);

    Ok(())
}

/// Offers to append the generated account to the configuration file
/// already there, printing it instead when the offer is declined.
fn append_or_print(
    printer: &mut impl Printer,
    path: &Path,
    config: ConfigureOutput,
    snapshot: &ConfigSnapshot,
) -> Result<()> {
    let prompt = format!("Append account `{}` to {}?", config.name, path.display());

    if !prompt::bool(prompt, true)? {
        return printer.out(config);
    }

    // NOTE: appending text keeps every comment and hand-written line as
    // they are, which re-serializing the document would not. The leading
    // newline separates the two tables, and terminates the last line of a
    // file that ends without one.
    write_configuration(path, &format!("\n{config}"), Some(snapshot))?;

    print_saved(path, &config);

    Ok(())
}

/// Writes or appends a generated account with private Unix permissions.
fn write_configuration(
    path: &Path,
    document: &str,
    snapshot: Option<&ConfigSnapshot>,
) -> Result<()> {
    let append = snapshot.is_some();
    if !append && let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        create_private_directories(parent)?;
    }

    let mut options = OpenOptions::new();
    options.write(true);
    if append {
        require_regular_path(path)?;
        options.read(true).append(true);
    } else {
        options.create_new(true);
    }
    #[cfg(unix)]
    options.mode(0o600).custom_flags(libc::O_NONBLOCK);
    let mut file = options
        .open(path)
        .with_context(|| format!("Open the config file {}", path.display()))?;
    require_regular_file(&file)?;
    if let Some(snapshot) = snapshot {
        let metadata = file.metadata()?;
        let original = snapshot.file.metadata()?;
        #[cfg(unix)]
        if metadata.dev() != original.dev() || metadata.ino() != original.ino() {
            bail!("Configuration was replaced while prompting");
        }
        #[cfg(not(unix))]
        if metadata.modified()? != original.modified()? {
            bail!("Configuration changed while prompting");
        }
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        if contents != snapshot.contents {
            bail!("Configuration changed while prompting");
        }
    }
    require_no_macos_acl(&file)?;
    #[cfg(unix)]
    if append {
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Restrict the config permissions {}", path.display()))?;
    }
    file.write_all(document.as_bytes())
        .with_context(|| format!("Write the config file {}", path.display()))?;
    Ok(())
}

/// The identity and bytes validated before the wizard prompts.
struct ConfigSnapshot {
    // NOTE: keeping the original inode open prevents its number being
    // reused by a replacement while the wizard is prompting.
    file: File,
    contents: String,
}

impl ConfigSnapshot {
    fn read(path: &Path) -> Result<Option<Self>> {
        match fs::metadata(path) {
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
            Ok(metadata) if !metadata.is_file() => bail!("Configuration is not a regular file"),
            Ok(_) => {}
        }
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        options.custom_flags(libc::O_NONBLOCK);
        let mut file = options.open(path)?;
        require_regular_file(&file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(Some(Self { file, contents }))
    }
}

fn require_regular_path(path: &Path) -> Result<()> {
    if !fs::metadata(path)?.is_file() {
        bail!("Configuration is not a regular file");
    }
    Ok(())
}

fn require_regular_file(file: &File) -> Result<()> {
    if !file.metadata()?.is_file() {
        bail!("Configuration is not a regular file");
    }
    Ok(())
}

/// Creates only missing directories and refuses inherited macOS ACLs.
fn create_private_directories(path: &Path) -> Result<()> {
    if path.is_dir() {
        return Ok(());
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        create_private_directories(parent)?;
    }
    let mut directory = DirBuilder::new();
    #[cfg(unix)]
    directory.mode(0o700);
    match directory.create(path) {
        Ok(()) => {
            #[cfg(target_os = "macos")]
            require_no_macos_acl(&File::open(path)?)?;
            Ok(())
        }
        Err(err) if err.kind() == ErrorKind::AlreadyExists && path.is_dir() => Ok(()),
        Err(err) => {
            Err(err).with_context(|| format!("Create the config directory {}", path.display()))
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn require_no_macos_acl(_file: &File) -> Result<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn require_no_macos_acl(file: &File) -> Result<()> {
    use std::{ffi::c_void, io, os::fd::AsRawFd, ptr};

    unsafe extern "C" {
        fn acl_get_fd_np(fd: libc::c_int, kind: libc::c_int) -> *mut c_void;
        fn acl_get_entry(acl: *mut c_void, id: libc::c_int, entry: *mut *mut c_void)
        -> libc::c_int;
        fn acl_free(acl: *mut c_void) -> libc::c_int;
    }
    // SAFETY: the descriptor stays live; ACL_TYPE_EXTENDED is 0x100 in
    // macOS sys/acl.h. The returned allocation is freed below.
    let acl = unsafe { acl_get_fd_np(file.as_raw_fd(), 0x100) };
    if acl.is_null() {
        let error = io::Error::last_os_error();
        // NOTE: macOS reports ENOENT when this live descriptor has no ACL.
        if error.raw_os_error() == Some(libc::ENOENT) {
            return Ok(());
        }
        return Err(error).context("Read configuration ACL");
    }
    let mut entry = ptr::null_mut();
    // SAFETY: acl is a valid allocation and entry is an out pointer.
    // ACL_FIRST_ENTRY is zero in macOS sys/acl.h.
    let result = unsafe { acl_get_entry(acl, 0, &mut entry) };
    let error = io::Error::last_os_error();
    // SAFETY: acl was allocated by acl_get_fd_np and is freed once.
    unsafe { acl_free(acl) };
    match result {
        0 => bail!("Configuration target has an ACL; choose an owner-only location"),
        // NOTE: Darwin returns EINVAL past the last entry of a valid ACL.
        -1 if error.raw_os_error() == Some(libc::EINVAL) => Ok(()),
        _ => Err(error).context("Inspect configuration ACL"),
    }
}

/// Tells where the account landed, under which name, and what to run
/// next.
///
/// The name matters because it was never asked for: an account that did
/// not claim the default is reachable through `-a` alone.
fn print_saved(path: &Path, config: &ConfigureOutput) {
    let name = &config.name;

    eprintln!();
    eprintln!("Account `{name}` saved to {}.", path.display());

    if !config.default {
        eprintln!("Another account holds the default, so name this one with `-a {name}`.");
    }

    eprintln!("Run `himalaya envelope list` to read your mailbox.");
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::*;
    use crate::config::{AccountConfig, MaildirConfig};

    static NEXT_CONFIG: AtomicUsize = AtomicUsize::new(0);

    /// A path in the temporary directory no other test writes to.
    fn config_path() -> PathBuf {
        let id = NEXT_CONFIG.fetch_add(1, Ordering::Relaxed);
        env::temp_dir().join(format!("himalaya-configure-{id}.toml"))
    }

    /// A minimal account naming a Maildir root, the one backend needing
    /// no network to describe.
    fn account(default: bool) -> AccountConfig {
        AccountConfig {
            default,
            maildir: Some(MaildirConfig {
                root: PathBuf::from("/tmp/mail"),
                keywords: Default::default(),
            }),
            ..Default::default()
        }
    }

    #[cfg(unix)]
    #[test]
    fn generated_configuration_and_new_directories_are_private() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("first/second/config.toml");
        write_configuration(&path, "password.raw = \"dummy\"\n", None).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        for parent in [dir.path().join("first"), dir.path().join("first/second")] {
            assert_eq!(
                fs::metadata(parent).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn appending_restricts_permissions_and_keeps_existing_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "# keep this comment\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        let snapshot = ConfigSnapshot::read(&path).unwrap().unwrap();
        write_configuration(&path, "new account\n", Some(&snapshot)).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# keep this comment\nnew account\n"
        );
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn saving_does_not_truncate_an_existing_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "original").unwrap();
        assert!(write_configuration(&path, "replacement", None).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
    }

    #[cfg(unix)]
    #[test]
    fn saving_refuses_a_dangling_symlink() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.toml");
        let path = dir.path().join("config.toml");
        symlink(&target, &path).unwrap();
        assert!(write_configuration(&path, "secret", None).is_err());
        assert!(!target.exists());
    }

    #[test]
    fn appending_refuses_changed_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "original").unwrap();
        let snapshot = ConfigSnapshot::read(&path).unwrap().unwrap();
        fs::write(&path, "changed").unwrap();
        assert!(write_configuration(&path, "secret", Some(&snapshot)).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"changed");
    }

    #[cfg(unix)]
    #[test]
    fn appending_refuses_a_replaced_inode_with_the_same_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "original").unwrap();
        let snapshot = ConfigSnapshot::read(&path).unwrap().unwrap();
        fs::rename(&path, dir.path().join("old.toml")).unwrap();
        fs::write(&path, "original").unwrap();
        assert!(write_configuration(&path, "secret", Some(&snapshot)).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
    }

    #[cfg(unix)]
    #[test]
    fn original_handle_survives_unlink_and_prevents_inode_reuse() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "original").unwrap();
        let snapshot = ConfigSnapshot::read(&path).unwrap().unwrap();
        let original = snapshot.file.metadata().unwrap();
        for _ in 0..32 {
            fs::remove_file(&path).unwrap();
            fs::write(&path, "original").unwrap();
            let replacement = fs::metadata(&path).unwrap();
            assert_ne!(replacement.ino(), original.ino());
            assert!(write_configuration(&path, "secret", Some(&snapshot)).is_err());
            assert_eq!(fs::read(&path).unwrap(), b"original");
        }
    }

    #[cfg(unix)]
    #[test]
    fn unchanged_symlink_configuration_can_be_appended() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.toml");
        let path = dir.path().join("config.toml");
        fs::write(&target, "original").unwrap();
        symlink(&target, &path).unwrap();
        let snapshot = ConfigSnapshot::read(&path).unwrap().unwrap();
        write_configuration(&path, "\nnew", Some(&snapshot)).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"original\nnew");
    }

    #[cfg(unix)]
    #[test]
    fn fifo_targets_are_rejected_without_waiting_for_a_reader() {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "original").unwrap();
        let snapshot = ConfigSnapshot::read(&path).unwrap().unwrap();
        fs::remove_file(&path).unwrap();
        let name = CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: name is a live, NUL-terminated temporary path.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(ExistingConfig::read(&path).is_err());
        assert!(write_configuration(&path, "secret", Some(&snapshot)).is_err());
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(libc::O_NONBLOCK);
        let mut reader = options.open(&path).unwrap();
        assert!(require_regular_file(&reader).is_err());
        assert!(write_configuration(&path, "secret", Some(&snapshot)).is_err());
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        assert!(bytes.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn explicit_acl_is_rejected_before_appending_or_chmod() {
        use std::process::Command;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "original").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(
            Command::new("/bin/chmod")
                .args(["+a", "everyone allow read"])
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        let snapshot = ConfigSnapshot::read(&path).unwrap().unwrap();
        assert!(write_configuration(&path, "secret", Some(&snapshot)).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o644
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn inherited_acls_are_rejected_before_writing_secrets() {
        use std::process::Command;
        let dir = tempfile::tempdir().unwrap();
        assert!(
            Command::new("/bin/chmod")
                .args([
                    "+a",
                    "everyone allow read,search,file_inherit,directory_inherit"
                ])
                .arg(dir.path())
                .status()
                .unwrap()
                .success()
        );
        let direct = dir.path().join("config.toml");
        assert!(write_configuration(&direct, "secret", None).is_err());
        assert_eq!(fs::read(&direct).unwrap(), b"");
        let nested = dir.path().join("first/second/config.toml");
        assert!(write_configuration(&nested, "secret", None).is_err());
        assert!(!nested.exists());
    }

    #[test]
    fn a_generated_account_parses_back() {
        let document = account(true).render("perso").expect("render the account");
        let config: Config = toml::from_str(&document).expect("parse the generated config");
        let account = &config.accounts["perso"];

        assert_eq!(config.accounts.len(), 1);
        assert!(account.default);
        assert_eq!(
            account.maildir.as_ref().map(|c| c.root.as_path()),
            Some(Path::new("/tmp/mail"))
        );

        // NOTE: a generated document holds what was configured, every
        // defaulted field being left out.
        assert!(!document.contains("imap"));
        assert!(!document.contains("table"));

        let lines: Vec<&str> = document.lines().collect();
        assert_eq!(lines[0], "[accounts.perso]");
        assert_eq!(lines[1], "default = true");
    }

    #[test]
    fn the_endpoint_reads_before_its_credentials() {
        // NOTE: serialized alphabetically the endpoint would sit under
        // its siblings, so the renderer lifts it to the top of its group.
        let document = account(true).render("perso").expect("render the account");
        let maildir: Vec<&str> = document
            .lines()
            .filter(|line| line.starts_with("maildir."))
            .collect();

        assert_eq!(maildir, ["maildir.root = \"/tmp/mail\""]);
    }

    #[test]
    fn an_appended_account_keeps_the_existing_one() {
        let path = config_path();

        // NOTE: no trailing newline, which is the shape an appended block
        // survives without merging into the last line.
        fs::write(
            &path,
            "# my accounts\n[accounts.work]\ndefault = true\nmaildir.root = \"/tmp/work\"",
        )
        .expect("write the existing config");

        let existing = ExistingConfig::read(&path)
            .expect("read the existing config")
            .expect("an existing config");

        assert_eq!(existing.names, ["work"]);
        assert!(existing.has_default);

        let document = account(!existing.has_default)
            .render("perso")
            .expect("render the account");
        let mut file = OpenOptions::new().append(true).open(&path).expect("open");
        write!(file, "\n{document}").expect("append the generated account");
        drop(file);

        let content = fs::read_to_string(&path).expect("read back");
        let config: Config = toml::from_str(&content).expect("parse the appended config");

        assert_eq!(config.accounts.len(), 2);

        let defaults = config
            .accounts
            .values()
            .filter(|account| account.default)
            .count();
        assert_eq!(defaults, 1);
        assert!(config.accounts["work"].default);
        assert!(content.starts_with("# my accounts"));

        fs::remove_file(&path).expect("remove the config");
    }

    #[test]
    fn a_taken_name_gets_a_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "").unwrap();
        let existing = ExistingConfig {
            names: vec!["perso".to_string(), "perso-2".to_string()],
            has_default: true,
            snapshot: ConfigSnapshot::read(&path).unwrap().unwrap(),
        };

        assert_eq!(account_name("perso", None), "perso");
        assert_eq!(account_name("perso", Some(&existing)), "perso-3");
        assert_eq!(account_name("work", Some(&existing)), "work");
    }

    #[test]
    fn a_missing_configuration_constrains_nothing() {
        let existing = ExistingConfig::read(&config_path()).expect("read a missing config");

        assert!(existing.is_none());
    }
}
