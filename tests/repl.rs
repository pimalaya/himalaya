//! # Repl
//!
//! `himalaya repl` run as the built binary against a Maildir, fed a piped
//! session, asserting that every line gets exactly one answer, that a
//! failing line leaves the session running, and that a command reading its
//! input off stdin is refused rather than swallowing the lines after it.

#![cfg(all(backend, feature = "maildir"))]

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// A Maildir account holding one seen message in its inbox as `m1`, and
/// the configuration reaching it.
fn account(dir: &Path) -> PathBuf {
    let root = dir.join("root");
    for sub in ["cur", "new", "tmp"] {
        fs::create_dir_all(root.join("inbox").join(sub)).unwrap();
    }
    fs::write(
        root.join("inbox/cur/m1:2,S"),
        "From: a@example.org\r\nSubject: Hello there\r\n\r\nbody\r\n",
    )
    .unwrap();

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

/// Runs a piped `--json` session over `lines` and returns one JSON value
/// per output line.
fn session(dir: &Path, lines: &str) -> Vec<Value> {
    let config = account(dir);
    let mut child = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap(), "--json", "repl"])
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(lines.as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );

    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn a_session_answers_each_line_and_survives_its_failures() {
    let dir = TempDir::new().unwrap();
    let answers = session(
        dir.path(),
        "envelope list\n\
         nope\n\
         flag add --flag flagged m1\n\
         envelope list \"unclosed\n\
         repl\n\
         envelope list\n\
         quit\n\
         envelope list\n",
    );

    assert_eq!(answers.len(), 6, "{answers:#?}");
    assert_eq!(answers[0]["envelopes"][0]["subject"], "Hello there");
    assert!(answers[1]["error"].is_string());
    assert!(answers[2].get("error").is_none(), "{}", answers[2]);
    assert!(answers[3]["error"].is_string());
    assert!(answers[4]["error"].is_string());

    let flags = answers[5]["envelopes"][0]["flags"].as_array().unwrap();
    assert!(
        flags.iter().any(|flag| flag["iana"] == "flagged"),
        "{flags:?}"
    );
}

#[test]
fn a_session_refuses_input_from_stdin() {
    let dir = TempDir::new().unwrap();
    let answers = session(
        dir.path(),
        "message add --mailbox inbox\n\
         message parse -\n\
         envelope list\n",
    );

    assert_eq!(answers.len(), 3, "{answers:#?}");
    assert_eq!(answers[0]["code"], "stdin-taken");
    assert_eq!(answers[1]["code"], "stdin-taken");
    assert_eq!(answers[2]["envelopes"][0]["id"], "m1");
}
