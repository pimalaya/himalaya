use std::path::Path;

use assert_cmd::Command;

/// Builds a `himalaya smtp` command with the given config path.
fn smtp(config: &Path) -> Command {
    let mut cmd = Command::cargo_bin("himalaya").unwrap();
    cmd.args(["-c", config.to_str().unwrap(), "smtp"]);
    cmd
}

/// Shared SMTP integration test suite.
///
/// Exercises the send command. Pass a path to a valid TOML config file
/// with a default SMTP account configured.
pub fn run(config: &Path, email: impl ToString) {
    let email = email.to_string();

    let eml = [
        &format!("From: Himalaya Test <{email}>"),
        &format!("To: Himalaya Test <{email}>"),
        "Subject: Himalaya SMTP integration test",
        "Date: Thu, 01 Jan 2026 00:00:00 +0000",
        "MIME-Version: 1.0",
        "Content-Type: text/plain; charset=utf-8",
        "",
        "This is a test email for himalaya SMTP integration tests.",
    ]
    .join("\r\n");

    // ── SEND message ──────────────────────────────────────────────────────

    smtp(config)
        .args(["messages", "send"])
        .write_stdin(eml.as_bytes())
        .assert()
        .success();
}
