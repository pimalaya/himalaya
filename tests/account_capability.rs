//! # Account capability
//!
//! `himalaya account capability` run as the built binary, asserting it
//! resolves each capability to the backend serving it without connecting:
//! the JMAP block points at a port nothing listens on and a credential
//! command that fails, so reaching the network would fail the command.

#![cfg(all(feature = "maildir", feature = "jmap"))]

use std::{
    fs,
    process::{Command, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// Runs `--json [args] account capability` for an account syncing a
/// Maildir from JMAP, its block extended with `extra`, returning the rows
/// by name.
fn capabilities(args: &[&str], extra: &str) -> Vec<Value> {
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("config.toml");
    let account = format!(
        "[accounts.a]\ndefault = true\nemail = \"a@example.org\"\nmaildir.root = \"{}\"\njmap.server = \"https://127.0.0.1:1\"\njmap.auth.bearer.token.command = \"false\"\n{extra}",
        dir.path().join("root").display()
    );
    fs::write(&config, account).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap(), "--json"])
        .args(args)
        .args(["account", "capability"])
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["capabilities"].as_array().unwrap().clone()
}

fn row<'a>(rows: &'a [Value], name: &str) -> &'a Value {
    rows.iter().find(|row| row["name"] == name).unwrap()
}

#[test]
fn each_capability_names_the_backend_serving_it() {
    let rows = capabilities(&[], "");

    assert_eq!(row(&rows, "mailbox.list")["backend"], "maildir");
    assert_eq!(row(&rows, "flags.important")["level"], "none");
    assert_eq!(row(&rows, "vacation")["backend"], "jmap");
    assert_eq!(row(&rows, "vacation")["level"], "server");
    assert_eq!(row(&rows, "submit")["backend"], Value::Null);
}

#[test]
fn a_backend_filter_shows_the_account_under_it() {
    let rows = capabilities(&["--backend", "jmap"], "");

    assert_eq!(row(&rows, "mailbox.list")["backend"], "jmap");
    assert_eq!(row(&rows, "mailbox.delete")["level"], "partial");
    assert_eq!(row(&rows, "submit")["backend"], "jmap");
}

#[test]
fn a_maildir_keeps_keywords_only_with_dovecot_keywords_on() {
    let rows = capabilities(&[], "maildir.keywords.header = \"x-keywords\"\n");
    assert_eq!(row(&rows, "flags.junk")["level"], "none");

    let rows = capabilities(&[], "maildir.keywords.dovecot = true\n");
    assert_eq!(row(&rows, "flags.important")["level"], "partial");
    assert_eq!(row(&rows, "flags.junk")["level"], "partial");
}
