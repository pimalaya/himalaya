//! # Message parse
//!
//! `message parse` run as the built binary, with `HOME` and
//! `XDG_CONFIG_HOME` pointing at an empty directory, so no configuration
//! can be found, against `message read` and `attachment download
//! --stdout` on a Maildir holding the same bytes.

#![cfg(backend)]

use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use tempfile::TempDir;

/// A message with a nested alternative, a Latin-1 body, a base64
/// attachment, an inline image and an attached message, its line endings
/// mixed as found in the wild.
const MIXED: &[u8] = b"From: =?utf-8?Q?Ren=C3=A9?= <rene@example.org>\r\n\
    To: a@example.org, Group: b@example.org, c@example.org;\r\n\
    Subject: =?iso-8859-1?Q?Caf=E9?=\r\n\
    Date: Tue, 06 Oct 2026 09:00:00 +0200\r\n\
    Message-ID: <1@example.org>\r\n\
    Content-Type: multipart/mixed; boundary=\"m\"\r\n\
    \r\n\
    --m\r\n\
    Content-Type: multipart/related; boundary=\"r\"\r\n\
    \r\n\
    --r\r\n\
    Content-Type: multipart/alternative; boundary=\"a\"\r\n\
    \r\n\
    --a\r\n\
    Content-Type: text/plain; charset=iso-8859-1\r\n\
    Content-Transfer-Encoding: quoted-printable\r\n\
    \r\n\
    Un caf=E9 ?\r\n\
    --a\r\n\
    Content-Type: text/html; charset=utf-8\r\n\
    \r\n\
    <p>Un caf\xc3\xa9 ?</p><img src=\"cid:logo@x\">\r\n\
    --a--\r\n\
    --r\r\n\
    Content-Type: image/png\r\n\
    Content-ID: <logo@x>\r\n\
    Content-Transfer-Encoding: base64\r\n\
    \r\n\
    iVBORw0KGgo=\r\n\
    --r--\r\n\
    --m\r\n\
    Content-Type: application/octet-stream\r\n\
    Content-Disposition: attachment; filename*=utf-8''r%C3%A9sum%C3%A9.bin\r\n\
    Content-Transfer-Encoding: base64\r\n\
    \r\n\
    AAH//gAKDQ==\r\n\
    --m\r\n\
    Content-Type: message/rfc822\n\
    \n\
    Subject: Inner\n\
    \n\
    inner body\n\
    --m--\r\n";

/// The binary, run with no configuration within reach.
struct Himalaya {
    home: TempDir,
}

impl Himalaya {
    fn new() -> Self {
        Self {
            home: TempDir::new().unwrap(),
        }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_himalaya"));
        cmd.args(args)
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path())
            .env_remove("HIMALAYA_CONFIG")
            .env_remove("XDG_CONFIG_DIRS")
            .stdin(Stdio::null());
        cmd
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    fn run_stdin(&self, args: &[&str], input: &[u8]) -> Output {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    }

    /// Writes a raw message beside the empty home.
    fn eml(&self, name: &str, raw: &[u8]) -> PathBuf {
        let path = self.home.path().join(name);
        fs::write(&path, raw).unwrap();
        path
    }
}

fn ok(output: Output) -> Vec<u8> {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output.stdout
}

fn json(output: Output) -> serde_json::Value {
    serde_json::from_slice(&ok(output)).unwrap()
}

#[test]
fn runs_with_no_configuration_from_a_path_and_from_stdin() {
    let himalaya = Himalaya::new();
    let path = himalaya.eml("m.eml", MIXED);
    let path = path.to_str().unwrap();

    let view = json(himalaya.run(&["--json", "message", "parse", path]));
    assert_eq!(view["headers"]["subject"], "Caf\u{e9}");
    assert_eq!(view["text"], "Un caf\u{e9} ?");
    let ids: Vec<&str> = view["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| part["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["4", "5", "6", "7", "8"]);

    let stdin = json(himalaya.run_stdin(&["--json", "message", "parse"], MIXED));
    let dash = json(himalaya.run_stdin(&["--json", "message", "parse", "-"], MIXED));
    assert_eq!(stdin, view);
    assert_eq!(dash, view);

    let text = ok(himalaya.run(&["message", "parse", path]));
    assert_eq!(ok(himalaya.run_stdin(&["message", "parse"], MIXED)), text);

    // NOTE: nothing was looked for, so nothing was offered or written.
    assert_eq!(fs::read_dir(himalaya.home.path()).unwrap().count(), 1);
}

#[test]
fn a_part_comes_out_as_its_decoded_bytes() {
    let himalaya = Himalaya::new();
    let path = himalaya.eml("m.eml", MIXED);
    let path = path.to_str().unwrap();

    let bytes = ok(himalaya.run(&["message", "parse", "--part", "7", path]));
    assert_eq!(bytes, b"\x00\x01\xff\xfe\x00\x0a\x0d");

    // NOTE: a Latin-1 body keeps its charset, only the transfer
    // encoding being undone.
    let body = ok(himalaya.run_stdin(&["message", "parse", "--part", "4"], MIXED));
    assert_eq!(body, b"Un caf\xe9 ?");

    let part = json(himalaya.run(&["--json", "message", "parse", "--part", "7", path]));
    assert_eq!(
        part,
        serde_json::json!({
            "id": "7",
            "mime": "application/octet-stream",
            "filename": "r\u{e9}sum\u{e9}.bin",
            "size": 7,
            "data": "AAH//gAKDQ==",
        })
    );
}

#[test]
fn an_unknown_or_container_part_is_refused() {
    let himalaya = Himalaya::new();
    let path = himalaya.eml("m.eml", MIXED);
    let path = path.to_str().unwrap();

    for id in ["1", "2", "9"] {
        let output = himalaya.run(&["--json", "message", "parse", "--part", id, path]);
        assert!(!output.status.success());
        let err: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let err = err["error"].as_str().unwrap();
        assert!(err.contains(&format!("No part with id {id}")), "{err}");
    }
}

#[test]
fn a_blank_source_is_refused_naming_it() {
    let himalaya = Himalaya::new();
    let path = himalaya.eml("blank.eml", b" \r\n\t\n");

    let output = himalaya.run(&["--json", "message", "parse", path.to_str().unwrap()]);
    assert!(!output.status.success());
    let err: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(err["error"].as_str().unwrap().contains("blank.eml"));

    let output = himalaya.run_stdin(&["--json", "message", "parse", "-"], b"\n\n");
    assert!(!output.status.success());
    let err: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(err["error"], "The message read from stdin is empty");
}

#[test]
fn a_message_too_complex_is_refused_with_its_code() {
    let himalaya = Himalaya::new();
    let mut raw = String::new();
    for n in 0..600 {
        raw.push_str(&format!("X-Line-{n}: {n}\r\n"));
    }
    raw.push_str("Content-Type: text/plain\r\n\r\nbody\r\n");

    let output = himalaya.run_stdin(&["--json", "message", "parse"], raw.as_bytes());
    assert!(!output.status.success());
    let err: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(err["code"], "message-too-complex");

    let output = himalaya.run_stdin(
        &["--json", "message", "parse", "--part", "1"],
        raw.as_bytes(),
    );
    let err: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(err["code"], "message-too-complex");
}

/// A Maildir account holding the given messages in its inbox, each
/// under its own id, and the configuration reaching it.
#[cfg(feature = "maildir")]
fn maildir(dir: &std::path::Path, messages: &[(&str, &[u8])]) -> PathBuf {
    let root = dir.join("root");
    for sub in ["cur", "new", "tmp"] {
        fs::create_dir_all(root.join("inbox").join(sub)).unwrap();
    }
    for (id, raw) in messages {
        fs::write(root.join("inbox/cur").join(format!("{id}:2,S")), raw).unwrap();
    }

    let config = dir.join("config.toml");
    fs::write(
        &config,
        format!(
            "[accounts.t]\ndefault = true\nemail = \"t@example.org\"\nmaildir.root = \"{}\"\n",
            root.display(),
        ),
    )
    .unwrap();
    config
}

#[cfg(feature = "maildir")]
#[test]
fn the_view_and_parts_are_those_of_message_read() {
    let himalaya = Himalaya::new();
    let store = TempDir::new().unwrap();
    let plain = b"From: a@example.org\nSubject: LF only\n\nbody\n";
    let config = maildir(store.path(), &[("mixed", MIXED), ("plain", plain)]);
    let config = config.to_str().unwrap();

    for (id, raw) in [("mixed", MIXED), ("plain", &plain[..])] {
        let path = himalaya.eml(&format!("{id}.eml"), raw);
        let path = path.to_str().unwrap();

        let read_json = ok(himalaya.run(&["-c", config, "--json", "message", "read", id]));
        let parse = ok(himalaya.run(&["--json", "message", "parse", path]));
        assert_eq!(parse, read_json, "{id}");

        let read = ok(himalaya.run(&["-c", config, "message", "read", id]));
        let parse = ok(himalaya.run_stdin(&["message", "parse"], raw));
        assert_eq!(parse, read, "{id}");

        let view: serde_json::Value = serde_json::from_slice(&read_json).unwrap();
        for part in view["parts"].as_array().unwrap() {
            let part = part["id"].as_str().unwrap();
            for json in [&[][..], &["--json"][..]] {
                let download = [
                    json,
                    &["-c", config, "attachment", "download", "--stdout", id, part],
                ]
                .concat();
                let parse = [json, &["message", "parse", "--part", part, path]].concat();
                assert_eq!(
                    ok(himalaya.run(&parse)),
                    ok(himalaya.run(&download)),
                    "{id} part {part} {json:?}",
                );
            }
        }
    }
}
