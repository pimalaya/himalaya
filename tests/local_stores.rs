//! # Local stores
//!
//! The leftovers of the 2026-11-13 Maildir and mbox campaign, run as the
//! built binary against throwaway stores: each command prints JSON, and
//! a refusal carries the code the spec names.

#![cfg(all(feature = "maildir", feature = "mbox"))]

use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// Runs `--json <args>` against the configuration `lines` written in
/// `dir`, returning whether it succeeded and the JSON printed.
fn himalaya(dir: &Path, lines: &str, args: &[&str]) -> (bool, Value) {
    let config = dir.join("config.toml");
    let account = format!("[accounts.a]\ndefault = true\nemail = \"a@example.org\"\n{lines}\n");
    fs::write(&config, account).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap(), "--json"])
        .args(args)
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();

    let json = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.success(), json)
}

fn maildir_at(path: &Path) {
    for subdir in ["cur", "new", "tmp"] {
        fs::create_dir_all(path.join(subdir)).unwrap();
    }
}

fn maildir_lines(root: &Path, extra: &str) -> String {
    format!("maildir.root = \"{}\"\n{extra}", root.display())
}

fn mbox_lines(root: &Path, extra: &str) -> String {
    format!("mbox.root = \"{}\"\n{extra}", root.display())
}

const MESSAGE: &str = "From a@b Mon Jan  1 00:00:00 2024\nSubject: one\n\nbody\n";

#[test]
fn a_maildir_role_alias_spelled_with_a_slash_marks_the_role() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    maildir_at(&root.join("Trash"));
    maildir_at(&root.join("Sent"));

    let lines = maildir_lines(
        &root,
        "mailbox.alias.trash = \"Trash/\"\nmailbox.alias.sent = \"./Sent/\"",
    );
    let (ok, out) = himalaya(dir.path(), &lines, &["mailbox", "list"]);
    assert!(ok, "{out}");

    let role = |name: &str| {
        let mailboxes = out["mailboxes"].as_array().unwrap();
        let mailbox = mailboxes.iter().find(|m| m["name"] == name).unwrap();
        mailbox["role"].clone()
    };
    assert_eq!(role("Trash"), "trash");
    assert_eq!(role("Sent"), "sent");
}

#[test]
fn a_raw_maildir_create_refuses_a_blank_or_control_name() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    maildir_at(&root);
    let lines = maildir_lines(&root, "");

    for name in [" ", "a\u{1}b", "Ok/ "] {
        let (ok, out) = himalaya(dir.path(), &lines, &["maildir", "create", name]);
        assert!(!ok, "{name}: {out}");
        let (ok, out) = himalaya(dir.path(), &lines, &["maildir", "create", "Ok"]);
        assert!(ok || out["error"].as_str().unwrap().contains("exists"));
        let (ok, _) = himalaya(dir.path(), &lines, &["maildir", "rename", "-m", "Ok", name]);
        assert!(!ok, "{name}");
    }

    let mut entries: Vec<String> = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(entries, ["Ok", "cur", "new", "tmp"]);
}

#[test]
fn a_maildirpp_rename_and_raw_delete_take_the_children_along() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    maildir_at(&root);
    maildir_at(&root.join(".Lists.rust"));
    maildir_at(&root.join(".Lists.rust.Kaffee"));
    let lines = maildir_lines(&root, "maildir.layout = \"maildir++\"");

    let (ok, out) = himalaya(
        dir.path(),
        &lines,
        &["mailbox", "rename", "Lists/rust", "go"],
    );
    assert!(ok, "{out}");
    assert!(root.join(".Lists.go.Kaffee/cur").is_dir());
    assert!(!root.join(".Lists.rust.Kaffee").exists());

    let (ok, out) = himalaya(dir.path(), &lines, &["mailbox", "delete", "Lists/go"]);
    assert!(!ok, "{out}");
    assert!(out["error"].as_str().unwrap().contains("Lists/go/Kaffee"));

    let (ok, out) = himalaya(dir.path(), &lines, &["maildir", "delete", "-m", "Lists/go"]);
    assert!(ok, "{out}");
    assert!(!root.join(".Lists.go").exists());
    assert!(!root.join(".Lists.go.Kaffee").exists());
}

#[test]
fn a_raw_maildir_move_within_one_folder_changes_nothing() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    maildir_at(&root.join("INBOX"));
    fs::write(root.join("INBOX/cur/1.host:2,S"), "Subject: a\n\nb\n").unwrap();
    let lines = maildir_lines(&root, "");

    let args = [
        "maildir", "message", "move", "-m", "INBOX", "-t", "./INBOX/", "1.host",
    ];
    let (ok, out) = himalaya(dir.path(), &lines, &args);
    assert!(ok, "{out}");
    assert_eq!(out["ids"], serde_json::json!(["1.host"]));
    assert!(root.join("INBOX/cur/1.host:2,S").is_file());
}

#[test]
fn an_unsearchable_maildir_names_the_permission_error() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    let locked = root.join("Locked");
    maildir_at(&locked);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    // NOTE: root searches through any mode, so the check only holds for
    // an ordinary user.
    let searchable = locked.join("cur").is_dir();
    let lines = maildir_lines(&root, "");
    let (ok, out) = himalaya(dir.path(), &lines, &["envelope", "list", "-m", "Locked"]);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    if searchable {
        return;
    }
    assert!(!ok);
    let err = out["error"].as_str().unwrap();
    assert!(err.contains("Permission denied"), "{err}");
    assert!(!err.contains("missing"), "{err}");
}

#[test]
fn a_directory_spool_is_left_out_of_the_listing() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(dir.path().join("spool")).unwrap();
    fs::write(root.join("A"), MESSAGE).unwrap();

    let spool = format!("mbox.inbox = \"{}\"", dir.path().join("spool").display());
    let lines = mbox_lines(&root, &spool);
    let (ok, out) = himalaya(dir.path(), &lines, &["mailbox", "list"]);
    assert!(ok, "{out}");
    let names: Vec<&str> = out["mailboxes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["A"]);
}

#[test]
fn a_file_that_is_no_mbox_is_refused_and_a_missing_one_coded() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("README.txt"), "read me\n").unwrap();
    let secret = dir.path().join("secret");
    fs::write(&secret, "root:x:0:0\n").unwrap();
    std::os::unix::fs::symlink(&secret, root.join("link")).unwrap();
    let message = dir.path().join("m.eml");
    fs::write(&message, "Subject: x\n\nb\n").unwrap();
    let message = message.to_str().unwrap();
    let lines = mbox_lines(&root, "");

    let (ok, out) = himalaya(
        dir.path(),
        &lines,
        &["envelope", "list", "-m", "README.txt"],
    );
    assert!(!ok);
    assert!(
        out["error"].as_str().unwrap().contains("is not an mbox"),
        "{out}"
    );

    let args = ["mbox", "message", "save", "-m", "link", message];
    let (ok, out) = himalaya(dir.path(), &lines, &args);
    assert!(!ok);
    assert!(
        out["error"].as_str().unwrap().contains("is not an mbox"),
        "{out}"
    );
    assert_eq!(fs::read_to_string(&secret).unwrap(), "root:x:0:0\n");

    for args in [
        &["envelope", "list", "-m", "Nope"][..],
        &["message", "add", "-m", "Nope", message],
    ] {
        let (ok, out) = himalaya(dir.path(), &lines, args);
        assert!(!ok);
        assert_eq!(out["code"], "mailbox-not-found", "{out}");
    }
}

#[test]
fn raw_mbox_writes_refuse_a_set_holding_an_unknown_id() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("A"), MESSAGE).unwrap();
    fs::write(root.join("B"), "").unwrap();
    let lines = mbox_lines(&root, "");

    let (_, out) = himalaya(dir.path(), &lines, &["envelope", "list", "-m", "A"]);
    let id = out["envelopes"][0]["id"].as_str().unwrap().to_owned();

    for args in [
        &[
            "mbox", "flag", "add", "-m", "A", "-f", "flagged", &id, "nope",
        ][..],
        &[
            "mbox", "flag", "set", "-m", "A", "-f", "flagged", &id, "nope",
        ],
        &[
            "mbox", "flag", "remove", "-m", "A", "-f", "seen", &id, "nope",
        ],
        &["mbox", "message", "copy", "-m", "A", "-t", "B", &id, "nope"],
        &["mbox", "message", "move", "-m", "A", "-t", "B", &id, "nope"],
        &["message", "read", "-m", "A", "nope"],
    ] {
        let (ok, out) = himalaya(dir.path(), &lines, args);
        assert!(!ok, "{args:?}");
        assert_eq!(out["code"], "message-not-found", "{args:?}: {out}");
    }
    assert_eq!(fs::read_to_string(root.join("A")).unwrap(), MESSAGE);
    assert_eq!(fs::read_to_string(root.join("B")).unwrap(), "");

    let args = ["mbox", "flag", "add", "-m", "A", "-f", "flagged", &id, &id];
    let (ok, out) = himalaya(dir.path(), &lines, &args);
    assert!(ok, "{out}");
    assert_eq!(out["ids"], serde_json::json!([id]));
}

#[test]
fn a_flag_set_keeps_the_thunderbird_keys_in_step() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    fs::create_dir_all(&root).unwrap();
    let mbox = "From - Mon Jan  1 00:00:00 2024\nX-Mozilla-Status: 0001\nX-Mozilla-Keys: $label1          \nSubject: one\n\nbody\n";
    fs::write(root.join("Tb"), mbox).unwrap();
    let lines = mbox_lines(&root, "");

    let (_, out) = himalaya(dir.path(), &lines, &["envelope", "list", "-m", "Tb"]);
    let id = out["envelopes"][0]["id"].as_str().unwrap().to_owned();

    let args = ["flag", "set", "-m", "Tb", "-f", "seen", &id];
    let (ok, out) = himalaya(dir.path(), &lines, &args);
    assert!(ok, "{out}");

    let contents = fs::read_to_string(root.join("Tb")).unwrap();
    assert!(!contents.contains("$label1"), "{contents}");
    assert!(contents.contains("X-Mozilla-Keys:\n"), "{contents}");
}
