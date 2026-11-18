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

/// A store holding `imap/INBOX` for the account `work`, the contacts of
/// the account `home`, and the configuration reaching it, `account` as
/// its `pimdir.account`.
fn account(dir: &Path, account: Option<&str>) -> PathBuf {
    let root = dir.join("store");
    fs::create_dir(&root).unwrap();
    let store = PimdirStore::open(&root).unwrap();
    store
        .for_account("work")
        .for_source("imap")
        .ensure_collection("imap/INBOX", MAIL)
        .unwrap();
    let store = PimdirStore::open(&root).unwrap();
    store
        .for_account("home")
        .for_source("carddav")
        .ensure_collection("carddav/contacts", "text/vcard")
        .unwrap();

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
    let config = account(dir.path(), None);

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
    let config = account(dir.path(), None);
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
    let config = account(dir.path(), Some("zzz"));
    let err = failure(&config, &["mailbox", "list"]);
    assert_eq!(err["code"], "store-account-unknown", "{err}");

    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    let config = account(dir.path(), None);
    PimdirStore::open(&root)
        .unwrap()
        .for_account("home")
        .for_source("imap")
        .ensure_collection("imap/Home", MAIL)
        .unwrap();
    let err = failure(&config, &["mailbox", "list"]);
    assert_eq!(err["code"], "store-account-ambiguous", "{err}");
}
