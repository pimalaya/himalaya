//! # IMAP Message-ID search
//!
//! The built binary against a scripted IMAP server that, like iCloud,
//! matches `HEADER Message-ID` only on the whole bracketed value: a `mid:`
//! search and the UID lookup of an `APPEND` answered without `APPENDUID`
//! both find their message.

#![cfg(feature = "imap")]

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
};

use serde_json::Value;
use tempfile::TempDir;

/// The one message of the scripted mailbox, UID 7.
const ID: &str = "mid-7@example.org";

/// The UID the scripted server gives an appended message.
const APPENDED: &str = "9";

/// A message carrying [`ID`], to append.
const MESSAGE: &str = "From: a@example.org\r\n\
    To: b@example.org\r\n\
    Subject: Found\r\n\
    Message-ID: <mid-7@example.org>\r\n\
    \r\n\
    hi\r\n";

/// Starts the scripted IMAP server and returns its address.
fn serve_imap() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            thread::spawn(move || imap_session(stream));
        }
    });

    addr
}

/// Answers one connection, with `SORT` and without `UIDPLUS`. A `SORT`
/// or `SEARCH` matches UID 7 for `INBOX` and UID [`APPENDED`] for `Sent`
/// when it holds `HEADER Message-ID <mid-7@example.org>`, nothing
/// otherwise.
fn imap_session(stream: TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut out = stream;
    let greeting = b"* PREAUTH [CAPABILITY IMAP4rev1 SORT] ready\r\n";
    if out.write_all(greeting).is_err() {
        return;
    }

    let bracketed = format!("<{ID}>");
    let mut selected = String::new();
    let mut line = String::new();

    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        let mut words = line.split_whitespace();
        let tag = words.next().unwrap_or_default().to_owned();
        let command: Vec<String> = words.map(|word| word.to_ascii_uppercase()).collect();
        let command: Vec<&str> = command.iter().map(String::as_str).collect();
        let hit = if selected == "SENT" { APPENDED } else { "7" };

        let reply = match command.as_slice() {
            ["CAPABILITY"] => format!("* CAPABILITY IMAP4rev1 SORT\r\n{tag} OK done\r\n"),
            ["SELECT", mailbox, ..] => {
                mailbox.trim_matches('"').clone_into(&mut selected);
                format!("* 1 EXISTS\r\n* OK [UIDVALIDITY 1] ok\r\n{tag} OK [READ-WRITE] done\r\n")
            }
            ["APPEND", ..] => {
                let literal = line.trim_end().rsplit('{').next().unwrap_or_default();
                let len: usize = literal.trim_end_matches(['}', '+']).parse().unwrap_or(0);
                if out.write_all(b"+ go\r\n").is_err() {
                    return;
                }
                let mut body = vec![0; len];
                if reader.read_exact(&mut body).is_err() {
                    return;
                }
                let mut rest = String::new();
                let _ = reader.read_line(&mut rest);
                format!("{tag} OK done\r\n")
            }
            ["UID", verb @ ("SORT" | "SEARCH"), ..] => {
                let words: Vec<&str> = line
                    .split_whitespace()
                    .map(|word| word.trim_matches(['"', '(', ')']))
                    .collect();
                let matched = words
                    .windows(2)
                    .any(|pair| pair[0].eq_ignore_ascii_case("Message-ID") && pair[1] == bracketed);
                if matched {
                    format!("* {verb} {hit}\r\n{tag} OK done\r\n")
                } else {
                    format!("* {verb}\r\n{tag} OK done\r\n")
                }
            }
            ["UID", "FETCH", ..] => format!(
                "* 1 FETCH (UID 7 FLAGS () RFC822.SIZE 100 ENVELOPE \
                 (\"Mon, 02 Mar 2026 10:00:00 +0000\" \"Found\" \
                 ((NIL NIL \"a\" \"example.org\")) ((NIL NIL \"a\" \"example.org\")) \
                 ((NIL NIL \"a\" \"example.org\")) ((NIL NIL \"b\" \"example.org\")) \
                 NIL NIL NIL \"<{ID}>\"))\r\n{tag} OK done\r\n"
            ),
            ["LIST", ..] => {
                format!("* LIST () \"/\" INBOX\r\n* LIST () \"/\" Sent\r\n{tag} OK done\r\n")
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

/// An IMAP account over the scripted server.
fn account(dir: &Path) -> PathBuf {
    let config = dir.join("config.toml");
    let toml = format!(
        "[accounts.a]\ndefault = true\nemail = \"a@example.org\"\n\
         imap.server = \"imap://{}\"\n",
        serve_imap(),
    );
    fs::write(&config, toml).unwrap();
    config
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

/// The JSON the run printed, asserting it succeeded.
fn success(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stdout}{stderr}");
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| panic!("{err}: {stderr}"))
}

/// A `mid:` search sends the bracketed id, the form every server matches.
#[test]
fn a_message_id_search_sends_the_bracketed_id() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());

    for query in [format!("mid:{ID}"), format!("mid:<{ID}>")] {
        let json = success(&run(&config, &["envelope", "search", &query]));
        let ids: Vec<&Value> = json["envelopes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|envelope| &envelope["id"])
            .collect();
        assert_eq!(ids, ["7"], "{query}: {json}");
    }
}

/// An `APPEND` answered without `APPENDUID` finds its UID by the
/// bracketed id.
#[test]
fn an_appended_uid_is_found_by_the_bracketed_id() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());

    let json = success(&run(&config, &["message", "add", "-m", "Sent", MESSAGE]));
    assert_eq!(json["id"], APPENDED, "{json}");
}
