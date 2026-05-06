use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// Cleanup resources allocated by the shared API test flow. The shared
/// API has no `mailboxes create/delete` of its own, so scaffolding goes
/// through the protocol-specific `imap mailboxes` commands. The shared
/// surface under test is everything else (`mailboxes list`,
/// `envelopes`, `messages`, `flags`, `attachments`).
struct Cleanup<'a> {
    config: &'a Path,
    mbox_name: Option<String>,
    mbox_name_2: Option<String>,
}

impl Drop for Cleanup<'_> {
    fn drop(&mut self) {
        if let Some(name) = &self.mbox_name {
            let _ = imap_scaffold(self.config)
                .args(["mailboxes", "delete", name])
                .output();
        }

        if let Some(name) = &self.mbox_name_2 {
            let _ = imap_scaffold(self.config)
                .args(["mailboxes", "delete", name])
                .output();
        }
    }
}

/// `himalaya <SHARED>` — invokes a shared subcommand. No `imap` /
/// `jmap` / `maildir` prefix; dispatch is driven by `--backend auto`
/// (configured account has IMAP only, so the shared command resolves
/// to the IMAP backend).
fn shared(config: &Path) -> Command {
    let mut cmd = Command::cargo_bin("himalaya").unwrap();
    cmd.args(["-c", config.to_str().unwrap()]);
    cmd
}

/// Same as `shared` but with `--json` so we can parse output.
fn shared_json(config: &Path) -> Command {
    let mut cmd = Command::cargo_bin("himalaya").unwrap();
    cmd.args(["--json", "-c", config.to_str().unwrap()]);
    cmd
}

/// Protocol-specific `himalaya imap …` used only for scaffolding
/// (create/delete mailboxes). The shared API has no equivalents.
fn imap_scaffold(config: &Path) -> Command {
    let mut cmd = Command::cargo_bin("himalaya").unwrap();
    cmd.args(["-c", config.to_str().unwrap(), "imap"]);
    cmd
}

/// Shared API integration test suite.
///
/// Exercises every command in the shared surface in a single ordered
/// flow against a backend that already works (here: Fastmail over
/// IMAP). The shared API has no mailbox create/delete, so the test
/// scaffolds setup/teardown via `imap mailboxes …` and exercises
/// every other shared subcommand on top of those mailboxes.
///
/// `messages send` is intentionally skipped: it needs SMTP or JMAP
/// and is covered by the dedicated `fastmail-smtp` / `fastmail-jmap`
/// test suites.
pub fn run(config: &Path, email: impl ToString) {
    let email = email.to_string();

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let mbox_name = format!("himalaya-shared-{ts}");
    let mbox_name_2 = format!("himalaya-shared-{ts}-copy");

    let mut cleanup = Cleanup {
        config,
        mbox_name: None,
        mbox_name_2: None,
    };

    // ── scaffold: two test mailboxes via the IMAP protocol-specific
    // commands; the shared API has no create/delete.
    imap_scaffold(config)
        .args(["mailboxes", "create", &mbox_name])
        .assert()
        .success();
    cleanup.mbox_name = Some(mbox_name.clone());

    imap_scaffold(config)
        .args(["mailboxes", "create", &mbox_name_2])
        .assert()
        .success();
    cleanup.mbox_name_2 = Some(mbox_name_2.clone());

    // ── 1. mailboxes list ────────────────────────────────────────────

    shared(config)
        .args(["mailboxes", "list"])
        .assert()
        .success();

    // `--counts` issues an extra STATUS per mailbox on IMAP (slow on
    // big accounts, but the test account is small).
    shared(config)
        .args(["mailboxes", "list", "--counts"])
        .assert()
        .success();

    // ── 2. messages add (with attachment) ────────────────────────────

    let eml = build_eml_with_attachment(&email);

    shared(config)
        .args([
            "messages",
            "add",
            "--mailbox",
            &mbox_name,
            "--flag",
            "draft",
        ])
        .write_stdin(eml.as_bytes())
        .assert()
        .success();

    // ── 3. envelopes list (plain, JSON for UID extraction) ───────────

    let stdout = shared_json(config)
        .args(["envelopes", "list", &mbox_name])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let envelopes: Vec<Value> = serde_json::from_slice::<Value>(&stdout)
        .unwrap_or_else(|e| {
            panic!(
                "failed to parse envelope list output: {e}\nstdout: {}",
                String::from_utf8_lossy(&stdout)
            )
        })
        .get("envelopes")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| {
            panic!(
                "missing `envelopes` key in output: {}",
                String::from_utf8_lossy(&stdout)
            )
        });

    assert_eq!(
        envelopes.len(),
        1,
        "expected exactly one envelope after `messages add`"
    );

    let uid = envelopes[0]["id"]
        .as_str()
        .map(|s| s.to_owned())
        .or_else(|| envelopes[0]["uid"].as_u64().map(|n| n.to_string()))
        .expect("envelope should expose an id");

    // ── 4. envelopes list flag variants ──────────────────────────────

    shared(config)
        .args(["envelopes", "list", &mbox_name, "--has-attachment"])
        .assert()
        .success();

    shared(config)
        .args(["envelopes", "list", &mbox_name, "--recipient"])
        .assert()
        .success();

    shared(config)
        .args([
            "envelopes",
            "list",
            &mbox_name,
            "--page",
            "1",
            "--page-size",
            "10",
        ])
        .assert()
        .success();

    // ── 5. messages get (default + --raw) ────────────────────────────

    shared(config)
        .args(["messages", "get", "--mailbox", &mbox_name, &uid])
        .assert()
        .success();

    let raw_stdout = shared(config)
        .args(["messages", "get", "--mailbox", &mbox_name, &uid, "--raw"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let raw_body = String::from_utf8_lossy(&raw_stdout);
    assert!(
        raw_body.contains("Subject:"),
        "`messages get --raw` should emit RFC 5322 bytes incl. Subject header, got: {raw_body}"
    );

    // ── 6. messages compose (stdout, no --send) ──────────────────────

    let compose_stdout = shared(config)
        .args([
            "messages",
            "compose",
            "--from",
            &email,
            "--to",
            &email,
            "--subject",
            "Himalaya shared compose",
            "--body",
            "compose body",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let compose_body = String::from_utf8_lossy(&compose_stdout);
    assert!(
        compose_body.contains("Subject: Himalaya shared compose"),
        "`messages compose` stdout should contain the assembled headers, got: {compose_body}"
    );

    // ── 7. flags add / set / remove ──────────────────────────────────

    shared(config)
        .args(["flags", "add", &mbox_name, &uid, "--flag", "seen"])
        .assert()
        .success();

    shared(config)
        .args([
            "flags", "set", &mbox_name, &uid, "--flag", "seen", "--flag", "flagged",
        ])
        .assert()
        .success();

    shared(config)
        .args(["flags", "remove", &mbox_name, &uid, "--flag", "flagged"])
        .assert()
        .success();

    // ── 8. attachments list (default + --inline) ─────────────────────

    let att_stdout = shared_json(config)
        .args(["attachments", "list", &mbox_name, &uid])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let att_body = String::from_utf8_lossy(&att_stdout);
    assert!(
        att_body.contains("hello.txt"),
        "`attachments list` should surface the test attachment filename, got: {att_body}"
    );

    shared(config)
        .args(["attachments", "list", &mbox_name, &uid, "--inline"])
        .assert()
        .success();

    // ── 9. attachments download ──────────────────────────────────────

    let dl_dir = TempDir::new().unwrap();
    shared(config)
        .args([
            "attachments",
            "download",
            &mbox_name,
            &uid,
            "--dir",
            dl_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();
    let downloaded = dl_dir.path().join("hello.txt");
    assert!(
        downloaded.exists(),
        "expected attachment to land at {}, dir listing: {:?}",
        downloaded.display(),
        fs::read_dir(dl_dir.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>()
    );

    // ── 10. messages copy ────────────────────────────────────────────

    shared(config)
        .args([
            "messages",
            "copy",
            "--from",
            &mbox_name,
            "--to",
            &mbox_name_2,
            &uid,
        ])
        .assert()
        .success();

    let stdout = shared_json(config)
        .args(["envelopes", "list", &mbox_name_2])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let dest_envelopes: Vec<Value> = serde_json::from_slice::<Value>(&stdout)
        .unwrap_or_else(|e| {
            panic!(
                "failed to parse destination envelope list: {e}\nstdout: {}",
                String::from_utf8_lossy(&stdout)
            )
        })
        .get("envelopes")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| {
            panic!(
                "missing `envelopes` key in destination output: {}",
                String::from_utf8_lossy(&stdout)
            )
        });
    assert_eq!(
        dest_envelopes.len(),
        1,
        "expected one message in copy destination"
    );
    let dest_uid = dest_envelopes[0]["id"]
        .as_str()
        .map(|s| s.to_owned())
        .or_else(|| dest_envelopes[0]["uid"].as_u64().map(|n| n.to_string()))
        .expect("destination envelope should expose an id");

    // ── 11. messages move ────────────────────────────────────────────

    shared(config)
        .args([
            "messages",
            "move",
            "--from",
            &mbox_name_2,
            "--to",
            &mbox_name,
            &dest_uid,
        ])
        .assert()
        .success();

    // cleanup via Drop
    let _ = cleanup;
}

/// Build a small multipart/mixed RFC 5322 message with a plain-text
/// body and a single named text attachment (`hello.txt`). Used to
/// exercise `attachments list` and `attachments download`.
fn build_eml_with_attachment(email: &str) -> String {
    let boundary = "HIMALAYA-SHARED-BOUNDARY";
    [
        &format!("From: Himalaya Shared <{email}>"),
        &format!("To: Himalaya Shared <{email}>"),
        "Subject: Himalaya shared API integration test",
        "Date: Thu, 01 Jan 2026 00:00:00 +0000",
        "MIME-Version: 1.0",
        &format!(r#"Content-Type: multipart/mixed; boundary="{boundary}""#),
        "",
        &format!("--{boundary}"),
        "Content-Type: text/plain; charset=utf-8",
        "",
        "This is the body for the shared API integration test.",
        "",
        &format!("--{boundary}"),
        r#"Content-Type: text/plain; charset=utf-8; name="hello.txt""#,
        r#"Content-Disposition: attachment; filename="hello.txt""#,
        "",
        "Attachment contents.",
        "",
        &format!("--{boundary}--"),
    ]
    .join("\r\n")
}
