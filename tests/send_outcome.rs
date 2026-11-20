//! # Send outcomes
//!
//! The built binary sending through a scripted SMTP server on the loopback,
//! beside a Maildir. A message that went out is never reported as a
//! failure: a copy the store then refuses is an item error of the receipt,
//! exit 3, and a reply whose source cannot be flagged answered prints one
//! receipt naming that. A message the server refuses is `smtp send`'s
//! result, exit 3.

#![cfg(all(feature = "smtp", feature = "maildir"))]

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
};

use serde_json::Value;
use tempfile::TempDir;

/// Starts an SMTP server accepting every connection and returns its
/// address. A recipient whose address starts with `refused` is refused.
fn serve() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            thread::spawn(move || session(stream));
        }
    });

    addr
}

/// Answers one connection, reading the message after `DATA` to its dot.
fn session(stream: TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut out = stream;
    if out.write_all(b"220 test ESMTP\r\n").is_err() {
        return;
    }

    let mut line = String::new();
    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        let command = line.to_ascii_uppercase();
        let reply = if command.starts_with("EHLO") {
            "250-test\r\n250 8BITMIME\r\n"
        } else if command.starts_with("RCPT TO:<REFUSED") {
            "550 5.1.1 No such user\r\n"
        } else if command.starts_with("DATA") {
            let _ = out.write_all(b"354 go\r\n");
            loop {
                line.clear();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == ".\r\n" {
                    break;
                }
            }
            "250 queued\r\n"
        } else if command.starts_with("QUIT") {
            let _ = out.write_all(b"221 bye\r\n");
            return;
        } else {
            "250 ok\r\n"
        };

        if out.write_all(reply.as_bytes()).is_err() {
            return;
        }
        line.clear();
    }
}

/// A Maildir account sending over the scripted server, its inbox holding
/// the source `plan` of a reply, and the configuration reaching both.
fn account(dir: &Path) -> PathBuf {
    let root = dir.join("root");
    for sub in ["cur", "new", "tmp"] {
        fs::create_dir_all(root.join("inbox").join(sub)).unwrap();
    }
    let source = b"From: alice@example.org\r\nTo: t@example.org\r\nSubject: Plan\r\n\
        Message-ID: <plan@example.org>\r\n\r\nhello\r\n";
    fs::write(root.join("inbox/cur/plan:2,S"), source).unwrap();

    let config = dir.join("config.toml");
    let lines = format!(
        "[accounts.t]\ndefault = true\nemail = \"t@example.org\"\n\
         maildir.root = \"{}\"\nsmtp.server = \"smtp://{}\"\n",
        root.display(),
        serve(),
    );
    fs::write(&config, lines).unwrap();
    config
}

fn run(config: &Path, args: &[&str]) -> Output {
    let home = config.parent().unwrap();
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

/// The one JSON value the run printed, and its exit status.
fn single(output: &Output) -> (Value, Option<i32>) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut values = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap());
    let value: Value = values.next().unwrap_or_default();
    assert!(values.next().is_none(), "{stdout}");
    (value, output.status.code())
}

const MESSAGE: &str = "From: t@example.org\r\nTo: bob@example.org\r\nSubject: Hi\r\n\r\nhi\r\n";

/// A message sent, whose copy the store then refuses, prints the receipt
/// of the send beside the item error of the copy, and exits 3.
#[test]
fn a_copy_refused_after_the_send_is_an_item_error() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());

    let output = run(&config, &["message", "send", "--save", "Nowhere", MESSAGE]);
    let (receipt, status) = single(&output);

    assert_eq!(status, Some(3), "{receipt}");
    assert_eq!(receipt["sent"], true, "{receipt}");
    assert_eq!(receipt["saved"], false, "{receipt}");
    assert_eq!(
        receipt["errors"][0]["code"], "mailbox-not-found",
        "{receipt}"
    );
    let error = receipt["errors"][0]["error"].as_str().unwrap();
    assert!(
        error.starts_with("Message sent, but saving a copy"),
        "{error}"
    );
}

/// A reply sent, whose source cannot be flagged answered, prints one
/// receipt naming the flag that failed, and exits 3.
#[test]
fn a_reply_whose_source_is_not_flagged_prints_one_receipt() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let cur = dir.path().join("root/inbox/cur");
    fs::set_permissions(&cur, fs::Permissions::from_mode(0o555)).unwrap();

    let args = [
        "message",
        "reply",
        "plan",
        "--send",
        "--no-save",
        "--body",
        "ok",
    ];
    let output = run(&config, &args);
    let (receipt, status) = single(&output);

    assert_eq!(status, Some(3), "{receipt}");
    assert_eq!(receipt["sent"], true, "{receipt}");
    assert_eq!(receipt["errors"][0]["id"], "plan", "{receipt}");

    fs::set_permissions(&cur, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A recipient the server refuses, the message reaching nobody, is the
/// result of `smtp send`, naming the reply, and exits 3.
#[test]
fn a_refused_smtp_send_is_the_result() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());

    let args = [
        "smtp",
        "send",
        "-f",
        "t@example.org",
        "-t",
        "refused@example.org",
        MESSAGE,
    ];
    let output = run(&config, &args);
    let (result, status) = single(&output);

    assert_eq!(status, Some(3), "{result}");
    let refusal = &result["refusals"][0];
    assert_eq!(refusal["rcptTo"], "<refused@example.org>", "{result}");
    assert_eq!(refusal["code"], 550, "{result}");
    assert_eq!(refusal["text"], "5.1.1 No such user", "{result}");
}

/// The files a Maildir folder holds, wherever it holds them.
fn held(folder: &Path) -> usize {
    ["cur", "new"]
        .iter()
        .map(|sub| fs::read_dir(folder.join(sub)).unwrap().count())
        .sum()
}

/// An MCP server allowed to send but not to write keeps no copy of what
/// it sends, whatever `message.send.save-copy` says, filing a copy being
/// a write; allowed to write too, it keeps the configured one.
#[test]
fn an_mcp_send_files_no_copy_unless_writes_are_allowed() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let sent = dir.path().join("root/Sent");
    for sub in ["cur", "new", "tmp"] {
        fs::create_dir_all(sent.join(sub)).unwrap();
    }
    let mut lines = fs::read_to_string(&config).unwrap();
    lines.push_str("message.send.save-copy = \"Sent\"\n");
    fs::write(&config, lines).unwrap();

    let send = |flags: &[&str]| {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "message_send", "arguments": { "message": MESSAGE } },
        });
        let mut child = Command::new(env!("CARGO_BIN_EXE_himalaya"))
            .args(["-c", config.to_str().unwrap(), "mcp"])
            .args(flags)
            .env("HOME", dir.path())
            .env("XDG_CONFIG_HOME", dir.path())
            .env_remove("XDG_CONFIG_DIRS")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(format!("{request}\n").as_bytes()).unwrap();
        drop(stdin);
        let output = child.wait_with_output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let response: Value = serde_json::from_str(stdout.trim()).unwrap();
        let result = response["result"].clone();
        assert_ne!(result["isError"], true, "{flags:?}: {response}");
        result["structuredContent"].clone()
    };

    let receipt = send(&["--allow-send"]);
    assert_eq!(receipt["sent"], true, "{receipt}");
    assert_eq!(receipt["saved"], false, "{receipt}");
    assert_eq!(held(&sent), 0);

    let receipt = send(&["--allow-send", "--allow-write"]);
    assert_eq!(receipt["saved"], true, "{receipt}");
    assert_eq!(held(&sent), 1);
}
