use std::{
    env,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use assert_cmd::Command;
use io_jmap::rfc8621::types::{
    email::Email, email_submission::EmailSubmission, identity::Identity, mailbox::Mailbox,
    thread::Thread, vacation_response::VacationResponse,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Resources to clean up after the test, even on failure.
struct Cleanup<'a> {
    config: &'a Path,
    /// Test mailbox ID — destroyed with --purge (removes all emails inside).
    mbox_id: Option<String>,
    /// Identity created during the test.
    identity_id: Option<String>,
}

impl Drop for Cleanup<'_> {
    fn drop(&mut self) {
        if let Some(id) = &self.identity_id {
            let _ = jmap(self.config).args(["identity", "delete", id]).output();
        }

        if let Some(id) = &self.mbox_id {
            let _ = jmap(self.config)
                .args(["mailboxes", "destroy", "--purge", id])
                .output();
        }
    }
}

/// Builds a `himalaya jmap` command with the given config path.
fn jmap(config: &Path) -> Command {
    let mut cmd = Command::cargo_bin("himalaya").unwrap();
    cmd.args(["-c", config.to_str().unwrap(), "jmap"]);
    cmd
}

/// Builds a `himalaya --json jmap` command (JSON output mode).
fn jmap_json(config: &Path) -> Command {
    let mut cmd = Command::cargo_bin("himalaya").unwrap();
    cmd.args(["--json", "-c", config.to_str().unwrap(), "jmap"]);
    cmd
}

/// Runs a JSON-mode command, asserts success, and deserializes stdout into T.
fn parse_output<T: DeserializeOwned>(config: &Path, args: &[&str]) -> T {
    let stdout = jmap_json(config)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    serde_json::from_slice(&stdout).unwrap_or_else(|e| {
        panic!(
            "failed to parse output for {:?}: {e}\nstdout: {}",
            args,
            String::from_utf8_lossy(&stdout)
        )
    })
}

/// Runs a JSON-mode command, asserts success, extracts `key` from the wrapper
/// object, and deserializes the value into `Vec<T>`.
fn parse_list<T: DeserializeOwned>(config: &Path, args: &[&str], key: &str) -> Vec<T> {
    let stdout = jmap_json(config)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&stdout).unwrap_or_else(|e| {
        panic!(
            "failed to parse output for {:?}: {e}\nstdout: {}",
            args,
            String::from_utf8_lossy(&stdout)
        )
    });

    serde_json::from_value(
        value
            .get(key)
            .cloned()
            .unwrap_or_else(|| panic!("missing `{key}` key in output for {args:?}: {value}")),
    )
    .unwrap_or_else(|e| {
        panic!("failed to deserialize `{key}` from output for {args:?}: {e}\nvalue: {value}")
    })
}

/// Shared JMAP integration test suite.
///
/// Exercises every command in a single ordered flow. Pass a path to a
/// valid TOML config file with a default JMAP account configured.
pub fn run(config: &Path, email: impl ToString) {
    let email = email.to_string();

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let mbox_name = format!("himalaya-test-{ts}");

    let mut cleanup = Cleanup {
        config,
        mbox_id: None,
        identity_id: None,
    };

    // ── 1. MAILBOXES ──────────────────────────────────────────────────────

    // baseline list — must return at least one mailbox (e.g. INBOX)
    let mboxes: Vec<Mailbox> = parse_list(config, &["mailboxes", "query"], "mailboxes");

    assert!(
        !mboxes.is_empty(),
        "mailboxes query should return at least one mailbox"
    );

    // create test mailbox (subscribed so it shows up in the default query)
    jmap(config)
        .args(["mailboxes", "create", &mbox_name, "--subscribe"])
        .assert()
        .success();

    // query by name — verify name matches
    let mboxes: Vec<Mailbox> = parse_list(
        config,
        &["mailboxes", "query", "--name", &mbox_name],
        "mailboxes",
    );

    assert_eq!(
        mboxes[0].name.as_deref(),
        Some(mbox_name.as_str()),
        "created mailbox name mismatch"
    );

    let mbox_id = mboxes[0].id.clone().expect("mailbox id");
    cleanup.mbox_id = Some(mbox_id.clone());

    // get by id — verify id and name
    let got: Vec<Mailbox> = parse_list(config, &["mailboxes", "get", &mbox_id], "mailboxes");

    assert_eq!(
        got[0].id.as_deref(),
        Some(mbox_id.as_str()),
        "get: id mismatch"
    );

    assert_eq!(
        got[0].name.as_deref(),
        Some(mbox_name.as_str()),
        "get: name mismatch"
    );

    // update: rename
    let mbox_name_2 = format!("{mbox_name}-renamed");

    jmap(config)
        .args(["mailboxes", "update", &mbox_id, "--name", &mbox_name_2])
        .assert()
        .success();

    // get by id again — verify the rename took effect
    let got: Vec<Mailbox> = parse_list(config, &["mailboxes", "get", &mbox_id], "mailboxes");

    assert_eq!(
        got[0].name.as_deref(),
        Some(mbox_name_2.as_str()),
        "mailbox rename not reflected in get"
    );

    // ── 2. EMAILS ─────────────────────────────────────────────────────────

    let eml = [
        &format!("From: Himalaya Test <{email}>"),
        &format!("To: Himalaya Test <{email}>"),
        "Subject: Himalaya integration test",
        "Date: Thu, 01 Jan 2026 00:00:00 +0000",
        "MIME-Version: 1.0",
        "Content-Type: text/plain; charset=utf-8",
        "",
        "This is a test email for himalaya integration tests.",
    ]
    .join("\r\n");

    // import from stdin
    jmap(config)
        .args(["emails", "import", "--mailbox-id", &mbox_id])
        .write_stdin(eml.as_bytes())
        .assert()
        .success();

    // query — verify exactly one email landed in the mailbox
    let emails: Vec<Email> = parse_list(
        config,
        &["emails", "query", "--mailbox", &mbox_id],
        "emails",
    );
    assert_eq!(emails.len(), 1, "expected exactly one email after import");

    let email_id = emails[0].id.clone().expect("email id");
    let thread_id = emails[0].thread_id.clone().expect("thread id");

    // get by id — verify the returned row matches the imported email
    let got: Vec<Email> = parse_list(config, &["emails", "get", &email_id], "emails");

    assert_eq!(
        got[0].id.as_deref(),
        Some(email_id.as_str()),
        "emails get: id mismatch"
    );

    // read: plain text — verify headers + body are present
    let stdout = jmap(config)
        .args(["emails", "read", &email_id])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(stdout).unwrap();

    assert!(
        text.contains("Himalaya integration test"),
        "read: subject missing"
    );

    assert!(text.contains("This is a test email"), "read: body missing");

    // read: html (no html part in fixture — command still succeeds)
    jmap(config)
        .args(["emails", "read", "--html", &email_id])
        .assert()
        .success();

    // update: add $seen — then verify via query with --has-keyword
    jmap(config)
        .args(["emails", "update", &email_id, "--add-keyword", "$seen"])
        .assert()
        .success();

    let seen: Vec<Email> = parse_list(
        config,
        &[
            "emails",
            "query",
            "--mailbox",
            &mbox_id,
            "--has-keyword",
            "$seen",
        ],
        "emails",
    );

    assert!(
        seen.iter()
            .any(|e| e.id.as_deref() == Some(email_id.as_str())),
        "email should have $seen keyword after update"
    );

    // update: add $flagged
    jmap(config)
        .args(["emails", "update", &email_id, "--add-keyword", "$flagged"])
        .assert()
        .success();

    // update: remove $flagged — then verify it is gone
    jmap(config)
        .args([
            "emails",
            "update",
            &email_id,
            "--remove-keyword",
            "$flagged",
        ])
        .assert()
        .success();

    let flagged: Vec<Email> = parse_list(
        config,
        &[
            "emails",
            "query",
            "--mailbox",
            &mbox_id,
            "--has-keyword",
            "$flagged",
        ],
        "emails",
    );

    assert!(
        !flagged
            .iter()
            .any(|e| e.id.as_deref() == Some(email_id.as_str())),
        "email should not have $flagged keyword after remove"
    );

    // export: raw RFC 5322 to stdout — verify original headers are present
    let stdout = jmap(config)
        .args(["emails", "export", &email_id])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let raw = String::from_utf8(stdout).unwrap();

    assert!(
        raw.contains("Subject: Himalaya integration test"),
        "export: subject missing"
    );

    assert!(
        raw.contains("From: Himalaya Test"),
        "export: From header missing"
    );

    // import --upload-only: upload blob and get its id
    let stdout = jmap(config)
        .args(["emails", "import", "--upload-only"])
        .write_stdin(eml)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let blob_id = String::from_utf8(stdout).unwrap().trim().to_owned();

    assert!(!blob_id.is_empty(), "upload-only must return a blob id");

    // parse the uploaded blob — verify subject is present in output
    let stdout = jmap(config)
        .args(["emails", "parse", &blob_id])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let body = String::from_utf8(stdout).unwrap();

    assert!(
        body.contains("This is a test email"),
        "parse: body missing from output"
    );

    // ── 3. THREADS ────────────────────────────────────────────────────────

    // get thread — verify it references the imported email
    let threads: Vec<Thread> = parse_list(config, &["threads", "get", &thread_id], "threads");

    assert_eq!(threads[0].id, thread_id, "thread: id mismatch");

    assert!(
        threads[0].email_ids.contains(&email_id),
        "thread should reference the imported email id"
    );

    // ── 4. IDENTITY ───────────────────────────────────────────────────────

    // create
    jmap(config)
        .args([
            "identity",
            "create",
            "Test",
            &email,
            "--text-signature",
            "Sent by himalaya integration tests",
        ])
        .assert()
        .success();

    // list — find by name and verify signature field
    let identities: Vec<Identity> = parse_list(config, &["identity", "get"], "identities");
    let identity = identities
        .iter()
        .find(|i| i.name == "Test")
        .expect("created identity not found in list");

    assert_eq!(
        identity.text_signature.as_deref(),
        Some("Sent by himalaya integration tests"),
        "identity textSignature mismatch after create"
    );

    let identity_id = identity.id.clone();
    let identity_email = identity.email.clone();
    cleanup.identity_id = Some(identity_id.clone());

    // update: rename
    jmap(config)
        .args(["identity", "update", &identity_id, "--name", "Test Updated"])
        .assert()
        .success();

    // list — verify rename
    let identities: Vec<Identity> = parse_list(config, &["identity", "get"], "identities");

    assert!(
        identities.iter().any(|i| i.name == "Test Updated"),
        "updated identity name not found in list"
    );

    // ── 5. SUBMISSION ─────────────────────────────────────────────────────

    // import a draft addressed to the account itself
    let draft = format!(
        "From: {identity_email}\r\n\
         To: {identity_email}\r\n\
         Subject: Himalaya submission test\r\n\
         Date: Thu, 01 Jan 2026 00:00:00 +0000\r\n\
         MIME-Version: 1.0\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         \r\n\
         Submission test by himalaya integration tests.\r\n"
    );

    jmap(config)
        .args([
            "emails",
            "import",
            "--mailbox-id",
            &mbox_id,
            "--keyword",
            "$draft",
        ])
        .write_stdin(draft.as_bytes())
        .assert()
        .success();

    // query to get draft id — verify it is flagged $draft
    let emails: Vec<Email> = parse_list(
        config,
        &[
            "emails",
            "query",
            "--mailbox",
            &mbox_id,
            "--has-keyword",
            "$draft",
        ],
        "emails",
    );

    assert!(!emails.is_empty(), "draft email not found after import");

    let draft_id = emails[0].id.clone().expect("draft id");

    // create submission (send) — JSON mode returns the created submission(s)
    let created: Vec<EmailSubmission> = parse_list(
        config,
        &[
            "submission",
            "create",
            &draft_id,
            "--identity-id",
            &identity_id,
        ],
        "submissions",
    );

    assert!(
        !created.is_empty(),
        "expected at least one created submission in response"
    );

    let sub_id = created[0].id.clone().expect("submission id");

    // get the submission by ID — EmailSubmission objects are short-lived on
    // some servers (e.g. Fastmail) and may already be gone by the time we
    // query; accept both found and not-found outcomes.
    let got: Vec<EmailSubmission> =
        parse_list(config, &["submission", "get", &sub_id], "submissions");

    if !got.is_empty() {
        assert_eq!(
            got[0].id.as_deref(),
            Some(sub_id.as_str()),
            "submission get: id mismatch"
        );
    }

    // ── 6. COPY (optional) ────────────────────────────────────────────────

    // Requires JMAP_FROM_ACCOUNT_ID env var (the server-side JMAP accountId,
    // e.g. "u1d764051" for FastMail). Set it to enable this step.
    if let Ok(from_account) = env::var("JMAP_FROM_ACCOUNT_ID") {
        let before: Vec<Email> = parse_list(
            config,
            &["emails", "query", "--mailbox", &mbox_id],
            "emails",
        );
        let count_before = before.len();

        jmap(config)
            .args([
                "emails",
                "copy",
                &email_id,
                "--from-account",
                &from_account,
                "--mailbox-id",
                &mbox_id,
            ])
            .assert()
            .success();

        let after: Vec<Email> = parse_list(
            config,
            &["emails", "query", "--mailbox", &mbox_id],
            "emails",
        );

        assert!(
            after.len() > count_before,
            "email copy should increase mailbox count"
        );
    }

    // ── 7. VACATION ───────────────────────────────────────────────────────

    // Check whether the server supports vacation response. Servers that do
    // not advertise the vacationresponse capability return a non-zero exit
    // code; in that case we skip the vacation assertions entirely.
    let vacation_supported = jmap_json(config)
        .args(["vacation", "get"])
        .output()
        .expect("failed to run vacation get")
        .status
        .success();

    if vacation_supported {
        // enable vacation response
        jmap(config)
            .args([
                "vacation",
                "set",
                "--enable",
                "--subject",
                "Away (himalaya test)",
                "--text-body",
                "I am away for himalaya integration testing.",
            ])
            .assert()
            .success();

        // verify enabled and subject
        let vacation: VacationResponse = parse_output(config, &["vacation", "get"]);

        assert!(
            vacation.is_enabled,
            "vacation should be enabled after set --enable"
        );

        assert_eq!(
            vacation.subject.as_deref(),
            Some("Away (himalaya test)"),
            "vacation subject mismatch"
        );

        // disable vacation response
        jmap(config)
            .args(["vacation", "set", "--disable"])
            .assert()
            .success();

        // verify disabled
        let vacation: VacationResponse = parse_output(config, &["vacation", "get"]);

        assert!(
            !vacation.is_enabled,
            "vacation should be disabled after set --disable"
        );
    }

    // ── 8. RAW QUERY ──────────────────────────────────────────────────────

    // raw Mailbox/get — shape is dynamic, use Value; verify response is a non-empty array
    let raw: Value = parse_output(
        config,
        &["query", r#"[["Mailbox/get", {"ids": null}, "c0"]]"#],
    );

    let responses = raw
        .get("method_responses")
        .and_then(|v| v.as_array())
        .expect("method_responses should be an array in raw query output");

    assert!(
        !responses.is_empty(),
        "raw query response should be a non-empty array"
    );

    // cleanup via Drop (identity delete + mailbox destroy --purge)
}
