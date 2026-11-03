//! # Mailbox
//!
//! The shared `mailbox create`, `rename` and `delete` run as the built
//! binary against a Maildir, asserting each changes the store and prints
//! the id it leaves.

#![cfg(feature = "maildir")]

use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// Runs `--json <args>` against a Maildir account rooted at `root`,
/// returning the JSON printed, failing on a non-zero exit.
fn himalaya(dir: &Path, root: &Path, args: &[&str]) -> Value {
    let config = dir.join("config.toml");
    let account = format!(
        "[accounts.a]\ndefault = true\nemail = \"a@example.org\"\nmaildir.root = \"{}\"\n",
        root.display()
    );
    fs::write(&config, account).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap(), "--json"])
        .args(args)
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env_remove("HIMALAYA_CONFIG")
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn a_mailbox_is_created_renamed_under_its_parent_and_deleted() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("root");
    for sub in ["cur", "new", "tmp"] {
        fs::create_dir_all(root.join("Archive").join(sub)).unwrap();
    }

    let created = himalaya(
        dir.path(),
        &root,
        &["mailbox", "create", "2025", "--parent", "Archive"],
    );
    assert_eq!(created["name"], "2025");
    assert!(root.join("Archive/2025/cur").is_dir(), "{created}");

    let renamed = himalaya(
        dir.path(),
        &root,
        &["mailbox", "rename", "Archive/2025", "2026"],
    );
    assert_eq!(renamed["name"], "2026");
    assert!(root.join("Archive/2026/cur").is_dir(), "{renamed}");
    assert!(!root.join("Archive/2025").exists(), "{renamed}");

    let deleted = himalaya(dir.path(), &root, &["mailbox", "delete", "Archive/2026"]);
    assert!(deleted["id"].is_string(), "{deleted}");
    assert!(!root.join("Archive/2026").exists(), "{deleted}");
}
