//! # Server refusals
//!
//! A whole command a server refused carries the code of that refusal at
//! every exit of the built binary: a one-shot run, a REPL line and an MCP
//! call. A scripted IMAP server refuses `CREATE` with `NOPERM` or
//! `OVERQUOTA` (RFC 5530 §3), the codes read off the typed error under
//! the context the command adds.

#![cfg(feature = "imap")]

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
};

use serde_json::{Value, json};
use tempfile::TempDir;

/// Starts an IMAP server answering every connection, the session
/// authenticated from its greeting, and returns its address.
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

/// Answers one connection: `CREATE Denied` refused with `NOPERM`,
/// `CREATE Full` with `OVERQUOTA`, anything else accepted.
fn session(stream: TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut out = stream;
    let greeting = b"* PREAUTH [CAPABILITY IMAP4rev1] ready\r\n";
    if out.write_all(greeting).is_err() {
        return;
    }

    let mut line = String::new();
    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        let mut words = line.split_whitespace();
        let tag = words.next().unwrap_or_default().to_owned();
        let command: Vec<&str> = words.collect();

        let reply = match command.as_slice() {
            ["CREATE", "Denied"] => format!("{tag} NO [NOPERM] Not yours\r\n"),
            ["CREATE", "Full"] => format!("{tag} NO [OVERQUOTA] Quota reached\r\n"),
            ["LIST", ..] => format!("* LIST () \"/\" INBOX\r\n{tag} OK done\r\n"),
            ["LOGOUT"] => format!("* BYE bye\r\n{tag} OK done\r\n"),
            _ => format!("{tag} OK done\r\n"),
        };

        if out.write_all(reply.as_bytes()).is_err() {
            return;
        }
        line.clear();
    }
}

/// An IMAP account over the scripted server, and the configuration
/// reaching it.
fn account(dir: &Path) -> PathBuf {
    let config = dir.join("config.toml");
    let toml = format!(
        "[accounts.a]\ndefault = true\nemail = \"a@example.org\"\n\
         imap.server = \"imap://{}\"\n",
        serve(),
    );
    fs::write(&config, toml).unwrap();
    config
}

/// Runs the binary over `config` with `args`, `input` on its stdin.
fn run(config: &Path, args: &[&str], input: &str) -> Output {
    let dir = config.parent().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap()])
        .args(args)
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input.as_bytes()).unwrap();
    drop(stdin);

    child.wait_with_output().unwrap()
}

/// The JSON values `output` printed, one per line.
fn values(output: &Output) -> Vec<Value> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|err| panic!("{err}: {stderr}")))
        .collect()
}

#[test]
fn a_one_shot_refusal_is_coded() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());

    for (name, code) in [("Denied", "permission-denied"), ("Full", "quota-exceeded")] {
        let output = run(&config, &["--json", "mailbox", "create", name], "");
        let printed = values(&output);
        assert_eq!(output.status.code(), Some(1), "{printed:?}");
        assert_eq!(printed[0]["code"], code, "{}", printed[0]);
    }
}

#[test]
fn a_refused_repl_line_is_coded() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let lines = "mailbox create Denied\nmailbox create Full\nmailbox create Fine\n";

    let output = run(&config, &["--json", "repl"], lines);
    let answers = values(&output);
    assert!(output.status.success(), "{answers:?}");

    assert_eq!(answers.len(), 3, "{answers:#?}");
    assert_eq!(answers[0]["code"], "permission-denied", "{}", answers[0]);
    assert_eq!(answers[1]["code"], "quota-exceeded", "{}", answers[1]);
    assert_eq!(answers[2]["id"], "Fine", "{}", answers[2]);
}

#[test]
fn a_refused_mcp_call_is_coded() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let calls: String = ["Denied", "Full"]
        .into_iter()
        .enumerate()
        .map(|(id, name)| {
            let call = json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": { "name": "mailbox_create", "arguments": { "name": name } },
            });
            format!("{call}\n")
        })
        .collect();

    let output = run(&config, &["mcp", "--allow-write"], &calls);
    let responses = values(&output);
    assert!(output.status.success(), "{responses:?}");

    assert_eq!(responses.len(), 2, "{responses:#?}");
    for (response, code) in responses
        .iter()
        .zip(["permission-denied", "quota-exceeded"])
    {
        let result = &response["result"];
        assert_eq!(result["isError"], true, "{response}");
        let report: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(report["code"], code, "{report}");
    }
}
