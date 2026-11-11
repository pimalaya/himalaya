//! # Envelope search
//!
//! `envelope search` run as the built binary against a Maildir, asserting
//! the copy, any-address, `before` and attachment clauses on the shared
//! client-side evaluator, and that a malformed query fails with the code
//! `search-invalid`.

#![cfg(all(backend, feature = "maildir"))]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// A message copied to Carol, sent on 2026-03-02, with no attachment.
const COPIED: &str = "From: alice@example.org\r\n\
    To: bob@example.org\r\n\
    Cc: Carol <carol@example.org>\r\n\
    Subject: Copied\r\n\
    Date: Mon, 02 Mar 2026 10:00:00 +0000\r\n\
    \r\n\
    hello\r\n";

/// A message to Bob alone, sent on 2026-05-04, carrying a PDF.
const ATTACHED: &str = "From: dave@example.org\r\n\
    To: bob@example.org\r\n\
    Subject: Attached\r\n\
    Date: Mon, 04 May 2026 10:00:00 +0000\r\n\
    Content-Type: multipart/mixed; boundary=\"m\"\r\n\
    \r\n\
    --m\r\n\
    Content-Type: text/plain\r\n\
    \r\n\
    see attached\r\n\
    --m\r\n\
    Content-Type: application/pdf\r\n\
    Content-Disposition: attachment; filename=\"a.pdf\"\r\n\
    \r\n\
    PDF\r\n\
    --m--\r\n";

/// A Maildir account holding both messages in its inbox, and the
/// configuration reaching it.
fn account(dir: &Path) -> PathBuf {
    let root = dir.join("root");
    for sub in ["cur", "new", "tmp"] {
        fs::create_dir_all(root.join("inbox").join(sub)).unwrap();
    }
    fs::write(root.join("inbox/cur/copied:2,S"), COPIED).unwrap();
    fs::write(root.join("inbox/cur/attached:2,S"), ATTACHED).unwrap();

    let config = dir.join("config.toml");
    fs::write(
        &config,
        format!(
            "[accounts.a]\ndefault = true\nemail = \"a@example.org\"\nmaildir.root = \"{}\"\n",
            root.display(),
        ),
    )
    .unwrap();
    config
}

/// Runs `envelope search --json` with the given query and returns the exit
/// status and the JSON it prints.
fn search(config: &Path, home: &Path, query: &[&str]) -> (bool, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args([
            "-c",
            config.to_str().unwrap(),
            "--json",
            "envelope",
            "search",
        ])
        .args(query)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();

    let json = serde_json::from_slice(&output.stdout).unwrap();
    (output.status.success(), json)
}

/// The subjects of a successful search, in the order printed.
fn subjects(config: &Path, home: &Path, query: &[&str]) -> Vec<String> {
    let (ok, json) = search(config, home, query);
    assert!(ok, "{json}");

    json["envelopes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|envelope| envelope["subject"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn new_clauses_select_their_messages() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let home = dir.path();

    assert_eq!(subjects(&config, home, &["cc", "carol"]), ["Copied"]);
    assert_eq!(subjects(&config, home, &["address", "dave"]), ["Attached"]);
    assert_eq!(subjects(&config, home, &["address", "carol"]), ["Copied"]);
    assert_eq!(
        subjects(&config, home, &["before", "2026-04-01"]),
        ["Copied"]
    );
    assert_eq!(subjects(&config, home, &["attachment"]), ["Attached"]);
    assert_eq!(subjects(&config, home, &["not", "attachment"]), ["Copied"]);
}

#[test]
fn a_malformed_query_is_refused_with_its_code() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let (ok, json) = search(&config, dir.path(), &["cc"]);

    assert!(!ok);
    assert_eq!(json["code"], "search-invalid", "{json}");
}
