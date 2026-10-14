//! # Message reply
//!
//! `message reply --all` run as the built binary against a Maildir
//! holding the source, printing the decoded template under `--json`.

#![cfg(all(backend, feature = "maildir"))]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use tempfile::TempDir;

/// A source addressed to the account (`t@example.org`) and others, with
/// a repeat, a name holding a comma and the account's address spelled in
/// another case in `Cc`.
const SOURCE: &[u8] = b"From: Alice <alice@example.org>\r\n\
    To: Me <t@example.org>, Bob <bob@example.org>, \"Carol, C\" <carol@example.org>\r\n\
    Cc: bob@example.org, Dave <dave@example.org>, T@Example.org\r\n\
    Subject: Plan\r\n\
    Message-ID: <plan@example.org>\r\n\
    \r\n\
    hello\r\n";

/// A Maildir account holding [`SOURCE`] in its inbox as `plan`, and the
/// configuration reaching it.
fn account(dir: &Path) -> PathBuf {
    let root = dir.join("root");
    for sub in ["cur", "new", "tmp"] {
        fs::create_dir_all(root.join("inbox").join(sub)).unwrap();
    }
    fs::write(root.join("inbox/cur/plan:2,S"), SOURCE).unwrap();

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

/// Runs `message reply` with the flags and returns the `to` and `cc`
/// lists of the template it prints.
fn reply(config: &Path, home: &Path, flags: &[&str]) -> (Vec<String>, Vec<String>) {
    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap(), "--json", "message", "reply"])
        .args(flags)
        .args(["--body", "ok", "plan"])
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("HIMALAYA_CONFIG")
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let template: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let list = |key: &str| {
        template[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect()
    };

    (list("to"), list("cc"))
}

#[test]
fn reply_all_copies_the_other_recipients() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());

    let (to, cc) = reply(&config, dir.path(), &[]);
    assert_eq!(to, ["Alice <alice@example.org>"]);
    assert!(cc.is_empty());

    for flag in ["--all", "-A"] {
        let (to, cc) = reply(&config, dir.path(), &[flag]);
        assert_eq!(to, ["Alice <alice@example.org>"], "{flag}");
        assert_eq!(
            cc,
            [
                "Bob <bob@example.org>",
                "\"Carol, C\" <carol@example.org>",
                "Dave <dave@example.org>",
            ],
            "{flag}",
        );
    }
}

#[test]
fn explicit_to_and_cc_win_over_the_derived_lists() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());

    let (to, cc) = reply(&config, dir.path(), &["--all", "--to", "bob@example.org"]);
    assert_eq!(to, ["bob@example.org"]);
    assert_eq!(
        cc,
        [
            "\"Carol, C\" <carol@example.org>",
            "Dave <dave@example.org>"
        ]
    );

    let (to, cc) = reply(&config, dir.path(), &["--all", "--cc", "erin@example.org"]);
    assert_eq!(to, ["Alice <alice@example.org>"]);
    assert_eq!(cc, ["erin@example.org"]);
}
