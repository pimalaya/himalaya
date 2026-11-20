//! # Early refusals
//!
//! A value a protocol command refuses itself is refused with the command
//! line: coded `usage-invalid`, exit 2, before the secret command runs and
//! the session opens. The account's password command writes a marker file,
//! which must not exist after the run.

#![cfg(all(
    feature = "imap",
    feature = "smtp",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph"
))]

use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn a_value_the_command_refuses_is_refused_before_the_session() {
    let dir = TempDir::new().unwrap();
    let marker = dir.path().join("secret-ran");
    let config = dir.path().join("config.toml");
    let command = format!("touch {}; echo secret", marker.display());
    let lines = format!(
        "[accounts.t]\ndefault = true\nemail = \"t@example.org\"\n\
         imap.server = \"imap://127.0.0.1:1\"\n\
         imap.sasl.plain.username = \"t\"\n\
         imap.sasl.plain.password.command = \"{command}\"\n\
         smtp.server = \"smtp://127.0.0.1:1\"\n\
         smtp.sasl.plain.username = \"t\"\n\
         smtp.sasl.plain.password.command = \"{command}\"\n",
    );
    fs::write(&config, lines).unwrap();

    for args in [
        &["imap", "fetch", "-m", "INBOX", "abc"][..],
        &["imap", "store", "abc", "-f", "\\Seen"],
        &["imap", "raw", "--", "NOOP"],
        &["imap", "raw", "--", "a1 NOOP\\na1 NOOP"],
        &["smtp", "send", "-f", "t@example.org", "-t", "b@example.org"],
        &[
            "smtp",
            "send",
            "--notify",
            "never,success",
            "-f",
            "t@example.org",
        ],
        &["jmap", "submission", "create", "E1", "--identity-id", "I1"],
        &["msgraph", "messages", "update", "M1"],
        &["msgraph", "request", "frob", "me"],
    ] {
        let args = refused(args);
        let output = run(dir.path(), &config, &args);
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert_eq!(output.status.code(), Some(2), "{args:?}: {stdout}");
        let json: Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(json["code"], "usage-invalid", "{args:?}: {json}");
        assert!(!marker.exists(), "{args:?} ran the secret command");
    }
}

/// A URL off the API a request command's token is for is refused coded
/// `request-origin`, exit 1, before the session opens.
#[test]
fn a_request_off_its_api_is_refused_before_the_session() {
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("config.toml");
    let lines = "[accounts.t]\ndefault = true\nemail = \"t@example.org\"\n\
                 gmail.auth.token.command = [\"false\"]\n\
                 msgraph.auth.token.command = [\"false\"]\n";
    fs::write(&config, lines).unwrap();

    for backend in ["msgraph", "gmail"] {
        let args = [backend, "request", "get", "https://evil.example.org/x"].map(String::from);
        let output = run(dir.path(), &config, &args);
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert_eq!(output.status.code(), Some(1), "{backend}: {stdout}");
        let json: Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(json["code"], "request-origin", "{backend}: {json}");
    }
}

/// The command line of `args` made refusable: a value only one of the
/// arguments carries is spliced in where a whole run needs it.
fn refused(args: &[&str]) -> Vec<String> {
    let mut args: Vec<String> = args.iter().map(ToString::to_string).collect();

    match args.as_slice() {
        [smtp, send, notify, ..] if smtp == "smtp" && send == "send" && notify == "--notify" => {
            args.extend(["-t".into(), "b@example.org".into(), "hi".into()]);
        }
        [smtp, send, ..] if smtp == "smtp" && send == "send" => {
            args.extend(["--notify".into(), "never,failure".into(), "hi".into()]);
        }
        [jmap, ..] if jmap == "jmap" => {
            args.extend(["--mail-from".into(), "t@example.org".into()]);
        }
        _ => {}
    }

    args
}

fn run(home: &Path, config: &Path, args: &[String]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .arg("-c")
        .arg(config)
        .arg("--json")
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}
