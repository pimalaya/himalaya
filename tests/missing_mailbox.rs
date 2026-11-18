//! # Missing IMAP mailboxes
//!
//! The built binary over a scripted IMAP server answering a mailbox it
//! does not hold as Fastmail does, a plain `NO`, or coded `NONEXISTENT`
//! or `TRYCREATE`. Every command addressing it fails coded
//! `mailbox-not-found`, the server's answer kept in `sources`, and a
//! plain `NO` costs one `LIST`, never sent before the command.

#![cfg(all(feature = "imap", feature = "smtp"))]

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use serde_json::Value;
use tempfile::TempDir;

/// The commands the scripted server was sent, one line each.
type Log = Arc<Mutex<Vec<String>>>;

/// Starts an IMAP server answering every connection, the session
/// authenticated from its greeting, and returns its address beside the
/// commands it is sent.
fn serve_imap() -> (String, Log) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let log = Log::default();
    let sent = log.clone();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            let sent = sent.clone();
            thread::spawn(move || imap_session(stream, sent));
        }
    });

    (addr, log)
}

/// The mailbox a command names, its quotes dropped.
fn name(word: &str) -> &str {
    word.trim_matches('"')
}

/// Answers one connection. `INBOX` and `Locked` are listed; `Nope` is
/// refused with a plain `NO`, `Gone` with `NONEXISTENT` and `Ghost` with
/// `TRYCREATE`; `Locked` is listed but refused with a plain `NO`.
fn imap_session(stream: TcpStream, log: Log) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut out = stream;
    let greeting = b"* PREAUTH [CAPABILITY IMAP4rev1 MOVE UIDPLUS] ready\r\n";
    if out.write_all(greeting).is_err() {
        return;
    }

    let refusal = |tag: &str, mailbox: &str| match name(mailbox) {
        "Gone" => Some(format!("{tag} NO [NONEXISTENT] Unknown mailbox\r\n")),
        "Ghost" => Some(format!("{tag} NO [TRYCREATE] Create it first\r\n")),
        "Nope" | "Locked" => Some(format!("{tag} NO Mailbox does not exist\r\n")),
        _ => None,
    };

    let mut line = String::new();
    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        log.lock().unwrap().push(line.trim_end().to_owned());
        let mut words = line.split_whitespace();
        let tag = words.next().unwrap_or_default().to_owned();
        let command: Vec<String> = words.map(|word| word.to_ascii_uppercase()).collect();
        let args: Vec<String> = line.split_whitespace().skip(2).map(String::from).collect();

        let reply = match command
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            ["CAPABILITY"] => {
                format!("* CAPABILITY IMAP4rev1 MOVE UIDPLUS\r\n{tag} OK done\r\n")
            }
            ["SELECT", ..] => refusal(&tag, &args[0]).unwrap_or_else(|| {
                format!("* 1 EXISTS\r\n* OK [UIDVALIDITY 1] ok\r\n{tag} OK [READ-WRITE] done\r\n")
            }),
            ["APPEND", ..] => {
                let literal = line.trim_end().rsplit('{').next().unwrap_or_default();
                let len: usize = literal.trim_end_matches('}').parse().unwrap_or(0);
                if out.write_all(b"+ go\r\n").is_err() {
                    return;
                }
                let mut body = vec![0; len];
                if reader.read_exact(&mut body).is_err() {
                    return;
                }
                let mut rest = String::new();
                let _ = reader.read_line(&mut rest);
                refusal(&tag, &args[0]).unwrap_or_else(|| format!("{tag} OK done\r\n"))
            }
            ["UID", "SEARCH", ..] => format!("* SEARCH 5\r\n{tag} OK done\r\n"),
            ["UID", "COPY" | "MOVE", ..] => {
                refusal(&tag, &args[2]).unwrap_or_else(|| format!("{tag} OK done\r\n"))
            }
            ["LIST", ..] => {
                format!("* LIST () \"/\" INBOX\r\n* LIST () \"/\" Locked\r\n{tag} OK done\r\n")
            }
            ["LOGOUT"] => format!("* BYE bye\r\n{tag} OK done\r\n"),
            _ => format!("{tag} OK done\r\n"),
        };

        if out.write_all(reply.as_bytes()).is_err() {
            return;
        }
        line.clear();
    }
}

/// Starts an SMTP server accepting every message and returns its address.
fn serve_smtp() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            thread::spawn(move || smtp_session(stream));
        }
    });

    addr
}

/// Answers one SMTP connection, reading the message after `DATA` to its
/// dot.
fn smtp_session(stream: TcpStream) {
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

/// An IMAP account over the scripted servers, the configuration
/// reaching it and the IMAP server's log.
fn account(dir: &Path) -> (PathBuf, Log) {
    let (imap, log) = serve_imap();
    let config = dir.join("config.toml");
    let toml = format!(
        "[accounts.a]\ndefault = true\nemail = \"a@example.org\"\n\
         imap.server = \"imap://{imap}\"\nsmtp.server = \"smtp://{}\"\n",
        serve_smtp(),
    );
    fs::write(&config, toml).unwrap();
    (config, log)
}

/// Runs the binary over `config` with `args`, under `--json`.
fn run(config: &Path, args: &[&str]) -> Output {
    let dir = config.parent().unwrap();
    Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap(), "--json"])
        .args(args)
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// The one JSON value the run printed, and its exit status.
fn single(output: &Output) -> (Value, Option<i32>) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let value = stdout
        .lines()
        .next()
        .map(|line| serde_json::from_str(line).unwrap())
        .unwrap_or_else(|| panic!("nothing printed: {stderr}"));
    (value, output.status.code())
}

/// How many `LIST` commands the server was sent.
fn lists(log: &Log) -> usize {
    log.lock()
        .unwrap()
        .iter()
        .filter(|line| line.split_whitespace().nth(1) == Some("LIST"))
        .count()
}

const MESSAGE: &str = "From: a@example.org\r\nTo: b@example.org\r\nSubject: Hi\r\n\r\nhi\r\n";

/// A `SELECT`, `APPEND`, `UID MOVE` or `UID COPY` the server refuses
/// for a mailbox it does not list fails coded `mailbox-not-found`, its
/// answer kept, whether it coded the refusal or not.
#[test]
fn a_missing_mailbox_is_coded() {
    let dir = TempDir::new().unwrap();
    let (config, _) = account(dir.path());

    let runs: [&[&str]; 7] = [
        &["envelope", "list", "-m", "Nope"],
        &["envelope", "list", "-m", "Gone"],
        &["message", "add", "-m", "Nope", MESSAGE],
        &["message", "add", "-m", "Ghost", MESSAGE],
        &["message", "move", "-f", "INBOX", "-t", "Nope", "5"],
        &["message", "copy", "-f", "INBOX", "-t", "Ghost", "5"],
        &["message", "copy", "-f", "INBOX", "-t", "Gone", "5"],
    ];

    for args in runs {
        let (failure, status) = single(&run(&config, args));
        assert_eq!(status, Some(1), "{args:?}: {failure}");
        assert_eq!(failure["code"], "mailbox-not-found", "{args:?}: {failure}");
        let sources = failure["sources"].to_string();
        assert!(sources.contains("IMAP"), "{args:?}: {failure}");
    }
}

/// A send whose copy the server refuses for a mailbox it does not list
/// is one receipt, the copy's item error coded `mailbox-not-found`.
#[test]
fn a_copy_to_a_missing_mailbox_is_coded() {
    let dir = TempDir::new().unwrap();
    let (config, _) = account(dir.path());

    let output = run(&config, &["message", "send", "--save", "Nope", MESSAGE]);
    let (receipt, status) = single(&output);

    assert_eq!(status, Some(3), "{receipt}");
    assert_eq!(receipt["sent"], true, "{receipt}");
    assert_eq!(receipt["saved"], false, "{receipt}");
    assert_eq!(
        receipt["errors"][0]["code"], "mailbox-not-found",
        "{receipt}"
    );
}

/// A plain `NO` for a mailbox the server lists stays the server's
/// uncoded refusal, and a listing that succeeds sends no `LIST`.
#[test]
fn a_listed_mailbox_keeps_the_refusal() {
    let dir = TempDir::new().unwrap();
    let (config, log) = account(dir.path());

    let (listed, status) = single(&run(&config, &["envelope", "list", "-m", "INBOX"]));
    assert_eq!(status, Some(0), "{listed}");
    assert_eq!(lists(&log), 0, "{:?}", log.lock().unwrap());

    let (failure, status) = single(&run(&config, &["envelope", "list", "-m", "Locked"]));
    assert_eq!(status, Some(1), "{failure}");
    assert_eq!(failure["code"], Value::Null, "{failure}");
    assert!(failure["error"].to_string().contains("NO"), "{failure}");
    assert_eq!(lists(&log), 1, "{:?}", log.lock().unwrap());
}
