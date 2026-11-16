//! # Local stores
//!
//! The leftovers of the 2026-11-13 Maildir and mbox campaign, run as the
//! built binary against throwaway stores: each command prints JSON, and
//! a refusal carries the code the spec names.

#![cfg(all(feature = "maildir", feature = "mbox"))]

use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// Runs `<args>` against the configuration `lines` written in `dir`.
fn run(dir: &Path, lines: &str, args: &[&str]) -> Output {
    let config = dir.join("config.toml");
    let account = format!("[accounts.a]\ndefault = true\nemail = \"a@example.org\"\n{lines}\n");
    fs::write(&config, account).unwrap();

    Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap()])
        .args(args)
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// Runs `--json <args>` against the configuration `lines` written in
/// `dir`, returning whether it succeeded and the JSON printed.
fn himalaya(dir: &Path, lines: &str, args: &[&str]) -> (bool, Value) {
    let output = run(dir, lines, &[&["--json"], args].concat());

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

    let args = ["mbox", "message", "add", "-m", "link", message];
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

#[test]
fn a_page_reaching_the_last_message_names_no_next_page() {
    let dir = TempDir::new().unwrap();
    let maildir = dir.path().join("maildir");
    maildir_at(&maildir.join("A"));
    fs::write(maildir.join("A/cur/one:2,S"), "Subject: one\n\nbody\n").unwrap();
    fs::write(maildir.join("A/cur/two:2,S"), "Subject: two\n\nbody\n").unwrap();

    let mbox = dir.path().join("mbox");
    fs::create_dir_all(&mbox).unwrap();
    let two = "From a@b Tue Jan  2 00:00:00 2024\nSubject: two\n\nbody\n";
    fs::write(mbox.join("A"), format!("{MESSAGE}\n{two}")).unwrap();

    for lines in [maildir_lines(&maildir, ""), mbox_lines(&mbox, "")] {
        for verb in ["list", "search"] {
            let page = |size: &str, page: &str| {
                let args = ["envelope", verb, "-m", "A", "-s", size, "-p", page];
                let (ok, out) = himalaya(dir.path(), &lines, &args);
                assert!(ok, "{out}");
                (
                    out["envelopes"].as_array().unwrap().len(),
                    out["nextPage"].clone(),
                )
            };

            assert_eq!(page("2", "1"), (2, Value::Null), "{verb} {lines}");
            assert_eq!(page("1", "1"), (1, Value::from("2")), "{verb} {lines}");
            assert_eq!(page("1", "2"), (1, Value::Null), "{verb} {lines}");
            assert_eq!(page("3", "1"), (2, Value::Null), "{verb} {lines}");
        }
    }

    let config = dir.path().join("config.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap()])
        .args(["envelope", "list", "-m", "A", "-s", "2"])
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env("XDG_CACHE_HOME", dir.path().join("cache"))
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{text}");
    assert!(text.contains("1, envelopes 1 to 2, the last"), "{text}");
    assert!(!text.contains("Next page:"), "{text}");
}

/// A Maildir file or an mbox message holding no header field is a
/// `message-unreadable` entry of `errors`, never a row, and the listing
/// exits 3, its readable messages listed.
#[test]
fn a_message_holding_no_header_field_is_an_item_error() {
    let dir = TempDir::new().unwrap();
    let maildir = dir.path().join("maildir");
    maildir_at(&maildir.join("A"));
    fs::write(maildir.join("A/cur/one:2,S"), "Subject: one\n\nbody\n").unwrap();
    fs::write(maildir.join("A/cur/empty:2,"), "").unwrap();
    fs::write(maildir.join("A/cur/text:2,"), "no header\nhere\n").unwrap();

    let mbox = dir.path().join("mbox");
    fs::create_dir_all(&mbox).unwrap();
    let text = "From a@b Tue Jan  2 00:00:00 2024\nno header\nhere\n";
    fs::write(mbox.join("A"), format!("{MESSAGE}\n{text}")).unwrap();

    let stores = [(maildir_lines(&maildir, ""), 2), (mbox_lines(&mbox, ""), 1)];
    for (lines, headerless) in stores {
        for verb in ["list", "search"] {
            let output = run(dir.path(), &lines, &["--json", "envelope", verb, "-m", "A"]);
            let out: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(output.status.code(), Some(3), "{verb} {lines}: {out}");

            let subjects: Vec<&str> = out["envelopes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|envelope| envelope["subject"].as_str().unwrap())
                .collect();
            assert_eq!(subjects, ["one"], "{verb} {lines}: {out}");

            let errors = out["errors"].as_array().unwrap();
            assert_eq!(errors.len(), headerless, "{verb} {lines}: {out}");
            for error in errors {
                assert_eq!(error["code"], "message-unreadable", "{out}");
                assert!(error["id"].is_string(), "{out}");
            }
        }

        let output = run(dir.path(), &lines, &["envelope", "list", "-m", "A"]);
        let text = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(3), "{text}");
        assert!(text.contains("message-unreadable"), "{text}");
        assert!(text.contains("1, envelope 1, the last"), "{text}");
    }
}

/// A read carries the id it was given and an `etag`, `null` on a local
/// store; an id the mailbox does not hold is `message-not-found`, on a
/// Maildir as on an mbox; a UTC date prints `Z`.
#[test]
fn a_read_carries_its_id_and_an_unknown_one_is_not_found() {
    let dir = TempDir::new().unwrap();
    let maildir = dir.path().join("maildir");
    maildir_at(&maildir.join("A"));
    let utc = "Date: Mon, 01 Jan 2024 10:00:00 +0000\nSubject: one\n\nbody\n";
    fs::write(maildir.join("A/cur/one:2,S"), utc).unwrap();

    let mbox = dir.path().join("mbox");
    fs::create_dir_all(&mbox).unwrap();
    let utc = format!("From a@b Mon Jan  1 00:00:00 2024\n{utc}");
    fs::write(mbox.join("A"), utc).unwrap();

    for lines in [maildir_lines(&maildir, ""), mbox_lines(&mbox, "")] {
        let (ok, out) = himalaya(dir.path(), &lines, &["envelope", "list", "-m", "A"]);
        assert!(ok, "{out}");
        let id = out["envelopes"][0]["id"].as_str().unwrap().to_owned();

        let (ok, out) = himalaya(dir.path(), &lines, &["message", "read", "-m", "A", &id]);
        assert!(ok, "{out}");
        assert_eq!(out["id"], id.as_str(), "{out}");
        assert_eq!(out["etag"], Value::Null, "{out}");
        assert_eq!(out["headers"]["date"], "2024-01-01T10:00:00Z", "{out}");

        let (ok, out) = himalaya(dir.path(), &lines, &["message", "read", "-m", "A", "999"]);
        assert!(!ok, "{out}");
        assert_eq!(out["code"], "message-not-found", "{lines}: {out}");
        let error = out["error"].as_str().unwrap();
        assert!(error.contains("`999`"), "{error}");
    }
}

/// A move onto its own mbox changes nothing and counts the messages, the
/// unified and the raw command alike, where io-mbox appended the copy,
/// dropped the original, then failed to find the copy it had written.
#[test]
fn an_mbox_move_onto_its_own_file_changes_nothing() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    fs::create_dir_all(&root).unwrap();
    let two = format!("{MESSAGE}\nFrom a@b Mon Jan  1 00:00:00 2024\nSubject: two\n\nbody\n");
    fs::write(root.join("A"), &two).unwrap();
    let lines = mbox_lines(&root, "");

    let (_, out) = himalaya(dir.path(), &lines, &["envelope", "list", "-m", "A"]);
    let id = out["envelopes"][1]["id"].as_str().unwrap().to_owned();

    let args = ["message", "move", "-f", "A", "-t", "A", &id];
    let (ok, out) = himalaya(dir.path(), &lines, &args);
    assert!(ok, "{out}");
    assert_eq!(out["count"], 1, "{out}");
    assert_eq!(fs::read_to_string(root.join("A")).unwrap(), two);

    let absolute = root.join("A");
    let absolute = absolute.to_str().unwrap();
    let args = ["mbox", "message", "move", "-m", "A", "-t", absolute, &id];
    let (ok, out) = himalaya(dir.path(), &lines, &args);
    assert!(ok, "{out}");
    assert_eq!(out["ids"], serde_json::json!([id]));
    assert_eq!(fs::read_to_string(root.join("A")).unwrap(), two);
}

/// The inbox of an mbox store without `mbox.inbox` is a mailbox that does
/// not exist, coded as one, as on Maildir.
#[test]
fn an_mbox_inbox_without_a_spool_is_not_found() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("A"), MESSAGE).unwrap();
    let lines = mbox_lines(&root, "");

    for args in [
        &["envelope", "list"][..],
        &["envelope", "list", "-m", "INBOX"],
    ] {
        let (ok, out) = himalaya(dir.path(), &lines, args);
        assert!(!ok, "{args:?}");
        assert_eq!(out["code"], "mailbox-not-found", "{args:?}: {out}");
        assert!(
            out["error"].as_str().unwrap().contains("mbox.inbox"),
            "{out}"
        );
    }
}

/// `mbox flag set` without a flag is refused, as `maildir flag set` is,
/// where it wiped every flag of the message.
#[test]
fn an_mbox_flag_set_without_a_flag_is_refused() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("store");
    fs::create_dir_all(&root).unwrap();
    let flagged =
        "From a@b Mon Jan  1 00:00:00 2024\nStatus: RO\nX-Status: F\nSubject: one\n\nbody\n";
    fs::write(root.join("A"), flagged).unwrap();
    let lines = mbox_lines(&root, "");

    let (_, out) = himalaya(dir.path(), &lines, &["envelope", "list", "-m", "A"]);
    let id = out["envelopes"][0]["id"].as_str().unwrap().to_owned();

    let output = run(
        dir.path(),
        &lines,
        &["--json", "mbox", "flag", "set", "-m", "A", &id],
    );
    assert_eq!(output.status.code(), Some(2));
    let out: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(out["code"], "usage-invalid", "{out}");
    assert_eq!(fs::read_to_string(root.join("A")).unwrap(), flagged);
}

/// A create naming a mailbox the store holds fails coded
/// `mailbox-duplicate`, exiting 1, unified and raw, on Maildir and mbox
/// alike, and the store is left as it was.
#[test]
fn a_mailbox_created_twice_is_a_duplicate() {
    let dir = TempDir::new().unwrap();
    let maildir = dir.path().join("maildir");
    maildir_at(&maildir);
    maildir_at(&maildir.join("Archive"));
    let mbox = dir.path().join("mbox");
    fs::create_dir_all(&mbox).unwrap();
    fs::write(mbox.join("Archive"), MESSAGE).unwrap();

    let stores = [
        (maildir_lines(&maildir, ""), "maildir"),
        (mbox_lines(&mbox, ""), "mbox"),
    ];
    for (lines, raw) in &stores {
        for args in [["mailbox", "create", "Archive"], [raw, "create", "Archive"]] {
            let output = run(dir.path(), lines, &[&["--json"], &args[..]].concat());
            let out: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(output.status.code(), Some(1), "{args:?}: {out}");
            assert_eq!(out["code"], "mailbox-duplicate", "{args:?}: {out}");
        }
    }

    assert_eq!(fs::read_to_string(mbox.join("Archive")).unwrap(), MESSAGE);
    let entries = fs::read_dir(maildir.join("Archive")).unwrap().count();
    assert_eq!(entries, 3);
}
