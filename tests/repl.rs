//! # Repl
//!
//! `himalaya repl` run as the built binary against a Maildir, fed a piped
//! session, asserting that every line gets exactly one answer, that a
//! failing line leaves the session running, that a command reading its
//! input off stdin is refused rather than swallowing the lines after it,
//! and that a line's `--json`, its comment, a session flag it sets and the
//! help it asks for are answered as pimalaya-cli's shared loop says.

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

/// Runs a piped session started with `flags` over `lines` and returns
/// its stdout and stderr.
fn run(dir: &Path, flags: &[&str], lines: &str) -> (String, String) {
    let config = account(dir);
    let mut child = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap()])
        .args(flags)
        .arg("repl")
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
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{stderr}");

    (String::from_utf8(output.stdout).unwrap(), stderr)
}

/// The JSON values of a piped session's stdout, one per line.
fn values(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|err| panic!("{err}: {line}")))
        .collect()
}

/// Runs a piped `--json` session over `lines` and returns one JSON value
/// per output line.
fn session(dir: &Path, lines: &str) -> Vec<Value> {
    values(&run(dir, &["--json"], lines).0)
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

#[test]
fn a_line_json_flag_switches_that_line_alone() {
    let dir = TempDir::new().unwrap();
    let (stdout, _) = run(
        dir.path(),
        &[],
        "envelope list --json\n\
         envelope list\n",
    );
    let mut lines = stdout.lines();

    let json: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert_eq!(json["envelopes"][0]["id"], "m1", "{stdout}");

    // The text list follows, nothing added between the two answers.
    let text: Vec<&str> = lines.collect();
    let expected = [
        "Account:    a (maildir)",
        "Mailbox:    inbox",
        "Page:       1, envelope 1, the last",
        "",
        "m1        -  a@example.org  Hello there",
    ];
    assert_eq!(text, expected, "{stdout}");
}

#[test]
fn a_session_flag_is_refused_by_name() {
    let dir = TempDir::new().unwrap();
    let answers = session(
        dir.path(),
        "envelope list -a other\n\
         envelope list -b maildir\n\
         envelope list --backend=imap\n\
         envelope list -c other.toml\n",
    );

    assert_eq!(answers.len(), 4, "{answers:#?}");

    for (answer, flag) in answers.iter().zip(["-a", "-b", "--backend", "-c"]) {
        let message =
            format!("Cannot set `{flag}` inside a session, start a new session with it instead");
        assert_eq!(answer["error"], message, "{answer}");
    }
}

#[test]
fn a_comment_line_is_skipped() {
    let dir = TempDir::new().unwrap();
    let answers = session(
        dir.path(),
        "# envelope list\n\
         \x20  # flag add --flag flagged m1\n\
         envelope list # all of them\n",
    );

    assert_eq!(answers.len(), 1, "{answers:#?}");
    assert_eq!(answers[0]["envelopes"][0]["id"], "m1");
}

#[test]
fn help_goes_to_stderr_in_a_json_session() {
    let dir = TempDir::new().unwrap();
    let (stdout, stderr) = run(
        dir.path(),
        &["--json"],
        "help\n\
         envelope list --help\n\
         envelope list\n",
    );

    let answers = values(&stdout);
    assert_eq!(answers.len(), 1, "{stdout}");
    assert_eq!(answers[0]["envelopes"][0]["id"], "m1");

    assert!(
        stderr.contains("Usage: himalaya [OPTIONS] <COMMAND>"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Usage: himalaya envelope list [OPTIONS]"),
        "{stderr}"
    );
}
