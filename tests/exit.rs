//! # Exit contract
//!
//! The built binary's exit statuses beyond success and failure: a command
//! line the grammar refuses exits 2, its failure object coded
//! `usage-invalid` under `--json`, and a command that printed its result
//! without fully succeeding exits 3.

use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// Runs the binary with `args` in `home`, its stdin closed.
fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn a_refused_command_line_under_json_prints_the_usage_failure_object() {
    let home = TempDir::new().unwrap();
    let output = run(home.path(), &["--json", "envelope", "list", "--nope"]);
    let stdout = text(&output.stdout);

    assert_eq!(output.status.code(), Some(2), "{stdout}");
    let json: Value = serde_json::from_str(&stdout).unwrap();
    let error = json["error"].as_str().unwrap();
    assert_eq!(json["code"], "usage-invalid", "{json}");
    assert!(error.starts_with("unexpected argument '--nope'"), "{error}");
    assert_eq!(json["sources"], Value::Array(Vec::new()), "{json}");
    assert!(output.stderr.is_empty(), "{}", text(&output.stderr));
}

#[test]
fn a_refused_command_line_without_json_is_clap_rendering_on_stderr() {
    let home = TempDir::new().unwrap();
    let output = run(home.path(), &["envelope", "list", "--nope"]);
    let stderr = text(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(output.stdout.is_empty(), "{}", text(&output.stdout));
    assert!(stderr.starts_with("error: unexpected argument"), "{stderr}");
}

/// A `--json` after `--` is a positional, not the flag.
#[test]
fn a_json_word_after_the_separator_leaves_the_refusal_on_stderr() {
    let home = TempDir::new().unwrap();
    let output = run(home.path(), &["envelope", "list", "--nope", "--", "--json"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "{}", text(&output.stdout));
    assert!(!output.stderr.is_empty());
}

#[cfg(feature = "maildir")]
#[test]
fn a_failed_account_check_prints_its_report_and_exits_3() {
    let home = TempDir::new().unwrap();
    let config = home.path().join("config.toml");
    let missing = home.path().join("missing");
    let toml = format!(
        "[accounts.a]\ndefault = true\nemail = \"a@example.org\"\nmaildir.root = {:?}\n",
        missing.display().to_string(),
    );
    std::fs::write(&config, toml).unwrap();

    let args = ["--json", "-c", config.to_str().unwrap(), "account", "check"];
    let output = run(home.path(), &args);
    let stdout = text(&output.stdout);

    assert_eq!(output.status.code(), Some(3), "{stdout}");
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["backends"][0]["backend"], "maildir", "{json}");
    assert_eq!(json["backends"][0]["ok"], false, "{json}");
}

/// A write given one id the mailbox holds and one it does not prints its
/// result with the unknown id as an item error, then exits 3.
#[cfg(feature = "maildir")]
#[test]
fn a_write_with_an_unknown_id_prints_its_errors_and_exits_3() {
    let home = TempDir::new().unwrap();
    let root = home.path().join("mail");
    for subdir in ["cur", "new", "tmp"] {
        std::fs::create_dir_all(root.join(subdir)).unwrap();
    }
    let message = b"From: a@example.org\r\nSubject: Hi\r\n\r\nbody\r\n";
    std::fs::write(root.join("cur/1700000000.1.host:2,"), message).unwrap();

    let config = home.path().join("config.toml");
    let toml = format!(
        "[accounts.a]\ndefault = true\nemail = \"a@example.org\"\nmaildir.root = {:?}\n",
        root.display().to_string(),
    );
    std::fs::write(&config, toml).unwrap();
    let config = config.to_str().unwrap();

    let output = run(home.path(), &["--json", "-c", config, "envelope", "list"]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["errors"], serde_json::json!([]), "{json}");
    let id = json["envelopes"][0]["id"].as_str().unwrap().to_owned();

    let args = [
        "--json", "-c", config, "flag", "add", "-f", "flagged", &id, "99",
    ];
    let output = run(home.path(), &args);
    let stdout = text(&output.stdout);
    assert_eq!(output.status.code(), Some(3), "{stdout}");
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["count"], 1, "{json}");
    assert_eq!(json["errors"][0]["id"], "99", "{json}");
    assert_eq!(json["errors"][0]["code"], "message-not-found", "{json}");

    let args = ["-c", config, "flag", "add", "-f", "flagged", "98", "99"];
    let output = run(home.path(), &args);
    let stdout = text(&output.stdout);
    assert_eq!(output.status.code(), Some(3), "{stdout}");
    assert_eq!(
        stdout,
        "Successfully added flags to 0 message(s): \\Flagged\n\n\
         Error:  98  message-not-found  Message not found in the mailbox\n\
         Error:  99  message-not-found  Message not found in the mailbox\n"
    );
}
