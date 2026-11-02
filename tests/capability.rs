//! # Capabilities
//!
//! The built binary refusing, under `capability-missing` or
//! `search-unsupported`, a shared verb the account's backend does not
//! serve, before it connects: every account below points at a credential
//! command that fails or a port nothing listens on, so reaching the
//! network would fail with another error.

#![cfg(all(
    feature = "gmail",
    feature = "msgraph",
    feature = "smtp",
    feature = "imap"
))]

use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// Runs `--json <args>` against a one-account configuration holding
/// `blocks`, returning the exit status and the JSON printed.
fn run(blocks: &str, args: &[&str]) -> (bool, Value) {
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("config.toml");
    let account = format!("[accounts.a]\ndefault = true\nemail = \"a@example.org\"\n{blocks}");
    fs::write(&config, account).unwrap();

    let output = himalaya(dir.path(), &config, args);
    let json = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
    (output.status.success(), json)
}

fn himalaya(home: &Path, config: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap(), "--json"])
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("HIMALAYA_CONFIG")
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

const GMAIL: &str = "gmail.auth.token.command = [\"false\"]\n";
const MSGRAPH: &str = "msgraph.auth.token.command = [\"false\"]\n";
const SMTP: &str = "smtp.server = \"127.0.0.1:1\"\n";
const IMAP: &str = "imap.server = \"imap://127.0.0.1:1\"\n";

/// Asserts a failure under `code` whose message holds `needle`.
fn refused(blocks: &str, args: &[&str], code: &str, needle: &str) {
    let (ok, json) = run(blocks, args);

    assert!(!ok, "{json}");
    assert_eq!(json["code"], code, "{json}");
    assert!(json.to_string().contains(needle), "{json}");
}

#[test]
fn gmail_refuses_answered_rather_than_report_it_done() {
    refused(
        GMAIL,
        &["flag", "add", "m1", "--flag", "answered"],
        "capability-missing",
        "flags.answered",
    );
}

#[test]
fn graph_refuses_adding_a_message() {
    refused(
        MSGRAPH,
        &[
            "message",
            "add",
            "-m",
            "INBOX",
            "--",
            "From: a@example.org\r\n\r\nx\r\n",
        ],
        "capability-missing",
        "message.add",
    );
}

#[test]
fn graph_refuses_a_send_whose_copy_it_cannot_hold() {
    refused(
        MSGRAPH,
        &[
            "message",
            "send",
            "--save",
            "Sent",
            "--",
            "From: a@example.org\r\nTo: b@example.org\r\n\r\nx\r\n",
        ],
        "capability-missing",
        "message.add",
    );
}

#[test]
fn a_send_only_account_refuses_the_storage_verbs() {
    refused(
        SMTP,
        &["envelope", "list"],
        "capability-missing",
        "configures no storage backend",
    );
}

#[test]
fn imap_refuses_an_attachment_clause_before_connecting() {
    refused(
        IMAP,
        &["envelope", "search", "attachment"],
        "search-unsupported",
        "attachment",
    );
}

#[test]
fn a_backend_filter_narrows_the_resolution() {
    refused(
        &format!("{IMAP}{SMTP}"),
        &["--backend", "smtp", "message", "read", "1"],
        "capability-missing",
        "`--backend smtp`",
    );
}
