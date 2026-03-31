use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use assert_cmd::Command;
use serde_json::Value;

/// Resources to clean up after the test, even on failure.
struct Cleanup<'a> {
    config: &'a Path,
    /// Primary test mailbox name — deleted on drop.
    mbox_name: Option<String>,
    /// Secondary test mailbox name (copy/move destination) — deleted on drop.
    mbox_name_2: Option<String>,
}

impl Drop for Cleanup<'_> {
    fn drop(&mut self) {
        if let Some(name) = &self.mbox_name {
            let _ = imap(self.config)
                .args(["mailboxes", "delete", name])
                .output();
        }

        if let Some(name) = &self.mbox_name_2 {
            let _ = imap(self.config)
                .args(["mailboxes", "delete", name])
                .output();
        }
    }
}

/// Builds a `himalaya imap` command with the given config path.
fn imap(config: &Path) -> Command {
    let mut cmd = Command::cargo_bin("himalaya").unwrap();
    cmd.args(["-c", config.to_str().unwrap(), "imap"]);
    cmd
}

/// Builds a `himalaya --json imap` command (JSON output mode).
fn imap_json(config: &Path) -> Command {
    let mut cmd = Command::cargo_bin("himalaya").unwrap();
    cmd.args(["--json", "-c", config.to_str().unwrap(), "imap"]);
    cmd
}

/// Shared IMAP integration test suite.
///
/// Exercises every command in a single ordered flow. Pass a path to a
/// valid TOML config file with a default IMAP account configured.
pub fn run(config: &Path, email: impl ToString) {
    let email = email.to_string();

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let mbox_name = format!("himalaya-test-{ts}");
    let mbox_name_renamed = format!("himalaya-test-{ts}-renamed");
    let mbox_name_2 = format!("himalaya-test-{ts}-copy");

    let mut cleanup = Cleanup {
        config,
        mbox_name: None,
        mbox_name_2: None,
    };

    // ── 1. LIST mailboxes ─────────────────────────────────────────────────

    // baseline list — must succeed and return output (INBOX always exists)
    imap(config).args(["mailboxes", "list"]).assert().success();

    // ── 2. CREATE mailbox ─────────────────────────────────────────────────

    imap(config)
        .args(["mailboxes", "create", &mbox_name])
        .assert()
        .success();

    cleanup.mbox_name = Some(mbox_name_renamed.clone());

    // create the copy/move destination mailbox
    imap(config)
        .args(["mailboxes", "create", &mbox_name_2])
        .assert()
        .success();

    cleanup.mbox_name_2 = Some(mbox_name_2.clone());

    // ── 3. RENAME mailbox ─────────────────────────────────────────────────

    imap(config)
        .args(["mailboxes", "rename", &mbox_name, &mbox_name_renamed])
        .assert()
        .success();

    // verify the renamed mailbox appears in list
    let stdout = imap(config)
        .args(["mailboxes", "list", "--all"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let list_output = String::from_utf8(stdout).unwrap();

    assert!(
        list_output.contains(&mbox_name_renamed),
        "renamed mailbox should appear in list"
    );

    // ── 4. STATUS ─────────────────────────────────────────────────────────

    imap(config)
        .args(["mailboxes", "status", &mbox_name_renamed])
        .assert()
        .success();

    // ── 5. APPEND a test message ──────────────────────────────────────────

    let eml = [
        &format!("From: Himalaya Test <{email}>"),
        &format!("To: Himalaya Test <{email}>"),
        "Subject: Himalaya IMAP integration test",
        "Date: Thu, 01 Jan 2026 00:00:00 +0000",
        "MIME-Version: 1.0",
        "Content-Type: text/plain; charset=utf-8",
        "",
        "This is a test email for himalaya IMAP integration tests.",
    ]
    .join("\r\n");

    imap(config)
        .args(["messages", "save", &mbox_name_renamed])
        .write_stdin(eml.as_bytes())
        .assert()
        .success();

    // ── 6. LIST envelopes ─────────────────────────────────────────────────

    let stdout = imap_json(config)
        .args(["envelopes", "list", "--mailbox", &mbox_name_renamed])
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
        "expected exactly one envelope after save"
    );

    let uid = envelopes[0]["uid"]
        .as_u64()
        .expect("envelope should have numeric uid");

    assert!(uid > 0, "uid must be non-zero");

    let uid_str = uid.to_string();

    // ── 7. GET the message ────────────────────────────────────────────────

    imap(config)
        .args(["messages", "get", "--mailbox", &mbox_name_renamed, &uid_str])
        .assert()
        .success();

    // ── 8. SEARCH ─────────────────────────────────────────────────────────

    let stdout = imap_json(config)
        .args([
            "envelopes",
            "search",
            "--mailbox",
            &mbox_name_renamed,
            "subject:Himalaya",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let results: Vec<Value> = serde_json::from_slice::<Value>(&stdout)
        .unwrap_or_else(|e| {
            panic!(
                "failed to parse search output: {e}\nstdout: {}",
                String::from_utf8_lossy(&stdout)
            )
        })
        .get("ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| {
            panic!(
                "missing `ids` key in search output: {}",
                String::from_utf8_lossy(&stdout)
            )
        });

    assert!(
        !results.is_empty(),
        "search should find the appended message"
    );

    // ── 9. FLAGS: add \\Seen, then remove it ──────────────────────────────

    imap(config)
        .args([
            "flags",
            "add",
            "--mailbox",
            &mbox_name_renamed,
            &uid_str,
            "--flag",
            "\\Seen",
        ])
        .assert()
        .success();

    imap(config)
        .args([
            "flags",
            "remove",
            "--mailbox",
            &mbox_name_renamed,
            &uid_str,
            "--flag",
            "\\Seen",
        ])
        .assert()
        .success();

    // ── 10. COPY message to the second mailbox ────────────────────────────

    imap(config)
        .args([
            "messages",
            "copy",
            "--mailbox",
            &mbox_name_renamed,
            &uid_str,
            &mbox_name_2,
        ])
        .assert()
        .success();

    // verify copy landed in destination
    let stdout = imap_json(config)
        .args(["envelopes", "list", "--mailbox", &mbox_name_2])
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

    let dest_uid = dest_envelopes[0]["uid"]
        .as_u64()
        .expect("destination envelope should have numeric uid");

    let dest_uid_str = dest_uid.to_string();

    // ── 11. MOVE message from second mailbox back to primary ──────────────

    imap(config)
        .args([
            "messages",
            "move",
            "--mailbox",
            &mbox_name_2,
            &dest_uid_str,
            &mbox_name_renamed,
        ])
        .assert()
        .success();

    // ── 12. DELETE both test mailboxes ────────────────────────────────────

    // delete the primary test mailbox
    imap(config)
        .args(["mailboxes", "delete", &mbox_name_renamed])
        .assert()
        .success();

    cleanup.mbox_name = None;

    // delete the secondary test mailbox
    imap(config)
        .args(["mailboxes", "delete", &mbox_name_2])
        .assert()
        .success();

    cleanup.mbox_name_2 = None;

    // cleanup via Drop (no-ops since we cleared both names above)
}
