//! # Envelope search
//!
//! `envelope search` run as the built binary against a Maildir, asserting
//! the `cc:`, `with:`, `date:`, `has:attachment` and `mid:` terms on
//! io-pimdir's mail matcher, and that a refused query, a reserved form
//! included, fails with the code `search-invalid`.

#![cfg(all(backend, feature = "maildir"))]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// A message copied to Carol, sent on 2026-03-02, with no attachment, its
/// body holding a word written with a tilde.
const COPIED: &str = "From: alice@example.org\r\n\
    To: bob@example.org\r\n\
    Cc: Carol <carol@example.org>\r\n\
    Subject: Copied\r\n\
    Message-ID: <copied-123@example.org>\r\n\
    Date: Mon, 02 Mar 2026 10:00:00 +0000\r\n\
    \r\n\
    hello ~tilde\r\n";

/// A message to Bob alone, sent on 2026-05-04, carrying a PDF.
const ATTACHED: &str = "From: dave@example.org\r\n\
    To: bob@example.org\r\n\
    Subject: Attached\r\n\
    Message-ID: <attached-456@example.org>\r\n\
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
/// code and the JSON it prints.
fn search(config: &Path, home: &Path, query: &[&str]) -> (Option<i32>, Value) {
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
    (output.status.code(), json)
}

/// The subjects of a successful search, in the order printed.
fn subjects(config: &Path, home: &Path, query: &[&str]) -> Vec<String> {
    let (code, json) = search(config, home, query);
    assert_eq!(code, Some(0), "{json}");

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

    assert_eq!(subjects(&config, home, &["cc:carol"]), ["Copied"]);
    assert_eq!(subjects(&config, home, &["with:dave"]), ["Attached"]);
    assert_eq!(subjects(&config, home, &["with:carol"]), ["Copied"]);
    assert_eq!(subjects(&config, home, &["date:..2026-03-31"]), ["Copied"]);
    assert_eq!(subjects(&config, home, &["has:attachment"]), ["Attached"]);
    assert_eq!(
        subjects(&config, home, &["not", "has:attachment"]),
        ["Copied"]
    );
    assert_eq!(
        subjects(&config, home, &["mid:copied-123@example.org"]),
        ["Copied"]
    );
    assert_eq!(
        subjects(&config, home, &["mid:<attached-456@example.org>"]),
        ["Attached"]
    );
    assert!(subjects(&config, home, &["mid:copied-123"]).is_empty());
    assert_eq!(
        subjects(&config, home, &["not", "mid:copied-123@example.org"]),
        ["Attached"]
    );
}

#[test]
fn a_malformed_query_is_refused_with_its_code() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let (code, json) = search(&config, dir.path(), &["cc:"]);

    assert_eq!(code, Some(1), "{json}");
    assert_eq!(json["code"], "search-invalid", "{json}");
}

/// A text beginning with `~` or `/` and a value beginning with `{` are
/// kept for later forms (pimdir SEARCH §8.3) and refused, where quoting
/// them searches the characters.
#[test]
fn a_reserved_form_is_refused_with_its_code() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let home = dir.path();

    for query in ["~tilde", "/slash", "subject:{brace}"] {
        let (code, json) = search(&config, home, &[query]);
        assert_eq!(code, Some(1), "{query}: {json}");
        assert_eq!(json["code"], "search-invalid", "{query}: {json}");
        let message = json.to_string();
        assert!(message.contains("reserved-form"), "{query}: {json}");
    }

    assert_eq!(subjects(&config, home, &["\"~tilde\""]), ["Copied"]);
}
