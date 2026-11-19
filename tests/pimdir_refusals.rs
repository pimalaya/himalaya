//! # pimdir refusals
//!
//! The refusals of the pimdir backend carry the codes the other pimdir
//! programs give them, run as the built binary over a store io-pimdir
//! fills: a root holding no store, an unknown mailbox, a non-numeric id,
//! an unknown queue row, a cancellation no terminal can confirm, a page
//! token the command did not print, and an account the store holds
//! several of, or none of.

#![cfg(feature = "pimdir")]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use io_pimdir::client::PimdirStore;
use serde_json::Value;
use tempfile::TempDir;

const MAIL: &str = "message/rfc822";

/// A collection the fixture declares: its account, source, name and
/// media type.
type Collection = (&'static str, &'static str, &'static str, &'static str);

/// `imap/INBOX` for the account `work` and the contacts of the account
/// `home`.
const WORK_AND_HOME: &[Collection] = &[
    ("work", "imap", "imap/INBOX", MAIL),
    ("home", "carddav", "carddav/contacts", "text/vcard"),
];

/// A store holding `collections` and the configuration reaching it,
/// `account` as its `pimdir.account`.
///
/// Every handle stays open until the last collection is declared, so
/// each open shares the owner lock the first took: a dropped lock may
/// still be held by a child another test is spawning, its copy of the
/// descriptor closed only at exec, and a reopen would find the store
/// owned.
fn account(dir: &Path, account: Option<&str>, collections: &[Collection]) -> PathBuf {
    let root = dir.join("store");
    fs::create_dir(&root).unwrap();

    let _held: Vec<_> = collections
        .iter()
        .map(|(account, source, collection, kind)| {
            let store = PimdirStore::open(&root)
                .unwrap()
                .for_account(*account)
                .for_source(*source);
            store.ensure_collection(collection, kind).unwrap();
            store
        })
        .collect();

    config(dir, &root, account)
}

fn config(dir: &Path, root: &Path, account: Option<&str>) -> PathBuf {
    let config = dir.join("config.toml");
    let mut lines = format!("[accounts.t]\ndefault = true\npimdir.root = {root:?}\n");
    if let Some(account) = account {
        lines.push_str(&format!("pimdir.account = {account:?}\n"));
    }
    fs::write(&config, lines).unwrap();
    config
}

/// The failure object a run printed, checked to exit 1.
fn failure(config: &Path, args: &[&str]) -> Value {
    let home = config.parent().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .arg("-c")
        .arg(config)
        .arg("--json")
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{args:?}: {stdout}");
    serde_json::from_str(stdout.trim()).unwrap()
}

#[test]
fn a_root_holding_no_store_is_uncreated() {
    let dir = TempDir::new().unwrap();
    let config = config(dir.path(), &dir.path().join("nowhere"), None);

    let err = failure(&config, &["envelope", "list"]);
    assert_eq!(err["code"], "store-uncreated", "{err}");
}

#[test]
fn the_refusals_of_a_store_carry_their_codes() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path(), None, WORK_AND_HOME);

    let cases: [(&[&str], &str); 5] = [
        (&["envelope", "list", "-m", "Nope"], "mailbox-not-found"),
        (
            &["message", "read", "-m", "imap/INBOX", "abc"],
            "message-not-found",
        ),
        (
            &["pimdir", "queue", "cancel", "999", "--yes"],
            "row-not-found",
        ),
        (&["pimdir", "queue", "cancel", "999"], "row-not-found"),
        (
            &["pimdir", "search", "--page-token", "zzz", "hi"],
            "page-token-invalid",
        ),
    ];

    for (args, code) in cases {
        let err = failure(&config, args);
        assert_eq!(err["code"], code, "{args:?}: {err}");
    }
}

/// Only the accounts holding mail count: the store sharing one mail
/// account with another's contacts needs no `pimdir.account`. One it
/// does not hold is refused rather than read as empty.
#[test]
fn the_account_is_the_one_holding_mail_or_refused() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path(), None, WORK_AND_HOME);
    let home = config.parent().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .arg("-c")
        .arg(&config)
        .args(["--json", "mailbox", "list"])
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("imap/INBOX"), "{stdout}");

    let dir = TempDir::new().unwrap();
    let config = account(dir.path(), Some("zzz"), WORK_AND_HOME);
    let err = failure(&config, &["mailbox", "list"]);
    assert_eq!(err["code"], "store-account-unknown", "{err}");

    let dir = TempDir::new().unwrap();
    let collections = [WORK_AND_HOME, &[("home", "imap", "imap/Home", MAIL)]].concat();
    let config = account(dir.path(), None, &collections);
    let err = failure(&config, &["mailbox", "list"]);
    assert_eq!(err["code"], "store-account-ambiguous", "{err}");
}

/// A configured account is read on a store holding no collection yet, a
/// first sync not having run: its mailbox set is empty, not refused.
#[test]
fn a_configured_account_is_read_on_an_empty_store() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    fs::create_dir(&root).unwrap();
    PimdirStore::open(&root).unwrap();
    let config = config(dir.path(), &root, Some("work"));
    let home = config.parent().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .arg("-c")
        .arg(&config)
        .args(["--json", "mailbox", "list"])
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    let mailboxes: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(mailboxes["mailboxes"], serde_json::json!([]), "{stdout}");
}
