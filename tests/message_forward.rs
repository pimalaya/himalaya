//! # Message forward
//!
//! `message forward` run as the built binary against a Maildir
//! holding the source, asserting that the attachments of the source
//! ride along under their own file names, that `--no-attachments`
//! leaves them out, and that the quoted body survives both. Regression
//! coverage for <https://github.com/pimalaya/himalaya/issues/780>.

#![cfg(all(backend, feature = "maildir"))]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use mail_parser::{MessageParser, MimeHeaders, PartType};
use tempfile::TempDir;

/// A source with a text body and one attachment, `résumé.bin`, carried
/// base64.
const SOURCE_WITH_ATTACHMENT: &[u8] = b"From: =?utf-8?Q?Ren=C3=A9?= <rene@example.org>\r\n\
    To: a@example.org\r\n\
    Subject: =?iso-8859-1?Q?Caf=E9?=\r\n\
    Date: Tue, 06 Oct 2026 09:00:00 +0200\r\n\
    Message-ID: <1@example.org>\r\n\
    Content-Type: multipart/mixed; boundary=\"m\"\r\n\
    \r\n\
    --m\r\n\
    Content-Type: text/plain; charset=utf-8\r\n\
    \r\n\
    see the attached resume\r\n\
    --m\r\n\
    Content-Type: application/octet-stream\r\n\
    Content-Disposition: attachment; filename*=utf-8''r%C3%A9sum%C3%A9.bin\r\n\
    Content-Transfer-Encoding: base64\r\n\
    \r\n\
    AAH//gAKDQ==\r\n\
    --m--\r\n";

/// A source with both bodies of an alternative pair and no attachment:
/// nothing to carry, the quoted form to keep.
const SOURCE_WITHOUT_ATTACHMENT: &[u8] = b"From: Alice <alice@example.org>\r\n\
    To: a@example.org\r\n\
    Subject: Both bodies\r\n\
    Message-ID: <4@example.org>\r\n\
    Content-Type: multipart/alternative; boundary=\"a\"\r\n\
    \r\n\
    --a\r\n\
    Content-Type: text/plain; charset=utf-8\r\n\
    \r\n\
    alternative plain\r\n\
    --a\r\n\
    Content-Type: text/html; charset=utf-8\r\n\
    \r\n\
    <p>alternative html</p>\r\n\
    --a--\r\n";

/// A source with an attachment part that names no file
/// (`Content-Disposition: attachment` without `filename`, no `name`
/// attribute): the part gets a fallback name, the quoted body survives.
const SOURCE_WITH_NAMELESS_ATTACHMENT: &[u8] = b"From: Alice <alice@example.org>\r\n\
    To: a@example.org\r\n\
    Subject: Nothing to carry\r\n\
    Message-ID: <5@example.org>\r\n\
    Content-Type: multipart/mixed; boundary=\"m\"\r\n\
    \r\n\
    --m\r\n\
    Content-Type: text/plain; charset=utf-8\r\n\
    \r\n\
    nothing here to attach\r\n\
    --m\r\n\
    Content-Type: application/octet-stream\r\n\
    Content-Disposition: attachment\r\n\
    \r\n\
    nameless bytes\r\n\
    --m--\r\n";

/// A source whose `multipart/mixed` container carries a `name`
/// attribute (`Content-Type: multipart/mixed; boundary="m"; name="weird"`):
/// without the container skip, `part::role`'s fallthrough classifies the
/// container itself as an attachment named "weird"; with the skip, nothing
/// is carried and the quoted body survives.
const SOURCE_WITH_NAMED_CONTAINER: &[u8] = b"From: Alice <alice@example.org>\r\n\
    To: a@example.org\r\n\
    Subject: Container named like an attachment\r\n\
    Message-ID: <6@example.org>\r\n\
    Content-Type: multipart/mixed; boundary=\"m\"; name=\"weird\"\r\n\
    \r\n\
    --m\r\n\
    Content-Type: text/plain; charset=utf-8\r\n\
    \r\n\
    the body inside the container\r\n\
    --m--\r\n";

/// The decoded bytes of the attachment of [`SOURCE_WITH_ATTACHMENT`],
/// as mail-builder writes them back base64-encoded.
const ATTACHMENT_BASE64: &str = "AAH//gAKDQ==";

/// A Maildir account holding `source` in its inbox as `src`, and the
/// configuration reaching it.
fn account(dir: &Path, source: &[u8]) -> PathBuf {
    let root = dir.join("root");
    for sub in ["cur", "new", "tmp"] {
        fs::create_dir_all(root.join("inbox").join(sub)).unwrap();
    }
    fs::write(root.join("inbox/cur/src:2,S"), source).unwrap();

    let config = dir.join("config.toml");
    fs::write(
        &config,
        format!(
            "[accounts.a]\ndefault = true\nemail = \"a@example.org\"\nmaildir.root = \"{}\"\n",
            root.display(),
        ),
    )
    .unwrap();
    config
}

/// Runs `message forward` of the source with the given extra flags and
/// returns the raw composed message it prints.
fn forward(config: &Path, home: &Path, flags: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap(), "message", "forward"])
        .args(flags)
        .args(["--body", "ok", "src"])
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

    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn forward_carries_the_attachment_of_the_source() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path(), SOURCE_WITH_ATTACHMENT);

    let composed = forward(&config, dir.path(), &[]);

    assert!(
        composed.contains("Content-Disposition: attachment; filename*=UTF-8''r%C3%A9sum%C3%A9.bin"),
        "the carried attachment lost its file name: {composed}"
    );
    assert!(
        composed.contains(ATTACHMENT_BASE64),
        "the carried attachment lost its bytes: {composed}"
    );
    assert!(
        composed.contains("> see the attached resume"),
        "the quoted source body was dropped: {composed}"
    );
}

#[test]
fn forward_keeps_the_quoted_body_alone_without_attachments() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path(), SOURCE_WITH_ATTACHMENT);

    let composed = forward(&config, dir.path(), &["--no-attachments"]);

    assert!(
        !composed.contains(ATTACHMENT_BASE64),
        "--no-attachments still carried the bytes: {composed}"
    );
    assert!(
        !composed.contains("résumé.bin"),
        "--no-attachments still carried the file name: {composed}"
    );
    assert!(
        composed.contains("> see the attached resume"),
        "the quoted source body was dropped: {composed}"
    );
}

#[test]
fn forward_of_a_source_without_attachment_keeps_the_text_form() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path(), SOURCE_WITHOUT_ATTACHMENT);

    let composed = forward(&config, dir.path(), &[]);

    assert!(
        composed.contains("> alternative plain"),
        "the quoted source body was dropped: {composed}"
    );
    assert!(
        !composed.contains("Content-Disposition: attachment"),
        "an attachment appeared out of nowhere: {composed}"
    );
}

#[test]
fn forward_carries_an_attachment_without_a_file_name() {
    for (mime, name) in [
        ("application/octet-stream", "attachment.bin"),
        ("application/pdf", "attachment.pdf"),
        ("application/x-himalaya-unknown", "attachment"),
    ] {
        let source = String::from_utf8_lossy(SOURCE_WITH_NAMELESS_ATTACHMENT)
            .replace("application/octet-stream", mime);
        let dir = TempDir::new().unwrap();
        let config = account(dir.path(), source.as_bytes());

        let composed = forward(&config, dir.path(), &[]);

        assert_carried(&composed, name, mime, b"nameless bytes");
        assert!(composed.contains("> nothing here to attach"), "{composed}");

        let suppressed = forward(&config, dir.path(), &["--no-attachments"]);
        assert!(!suppressed.contains("Content-Disposition: attachment"));
        assert!(suppressed.contains("> nothing here to attach"));
    }
}

#[test]
fn forward_names_an_attached_message_from_its_subject() {
    for (subject, name) in [
        ("Subject: Quarterly report\r\n", "Quarterly report.eml"),
        ("Subject: ../report\\draft:Q1?\r\n", "_report_draft_Q1_.eml"),
        ("Subject: =?utf-8?Q?caf=C3=A9?=\r\n", "café.eml"),
        ("", "attachment.eml"),
        ("Subject:   \r\n", "attachment.eml"),
        ("Subject: ... \r\n", "attachment.eml"),
    ] {
        let inner = format!(
            "From: Inner <inner@example.org>\r\n{subject}Content-Type: text/plain\r\n\r\nattached email body"
        );
        let source = String::from_utf8_lossy(SOURCE_WITH_NAMELESS_ATTACHMENT)
            .replace("application/octet-stream", "message/rfc822")
            .replace("nameless bytes", &inner);
        let dir = TempDir::new().unwrap();
        let config = account(dir.path(), source.as_bytes());

        let composed = forward(&config, dir.path(), &[]);

        assert_carried(&composed, name, "message/rfc822", inner.as_bytes());
        assert!(composed.contains("> nothing here to attach"), "{composed}");

        let suppressed = forward(&config, dir.path(), &["--no-attachments"]);
        assert!(!suppressed.contains("Content-Disposition: attachment"));
        assert!(!suppressed.contains("attached email body"));
        assert!(suppressed.contains("> nothing here to attach"));
    }
}

/// Checks the carried MIME part after decoding, including all its bytes.
fn assert_carried(composed: &str, name: &str, mime: &str, bytes: &[u8]) {
    let parsed = MessageParser::new().parse(composed.as_bytes()).unwrap();
    let mut attachments = parsed.attachments();
    let attachment = attachments.next().expect("nameless attachment was dropped");
    assert!(attachments.next().is_none(), "unexpected extra attachment");
    assert_eq!(attachment.attachment_name(), Some(name));
    let content_type = attachment.content_type().unwrap();
    assert_eq!(
        format!(
            "{}/{}",
            content_type.c_type,
            content_type.c_subtype.as_deref().unwrap()
        ),
        mime
    );
    let carried = match &attachment.body {
        PartType::Binary(bytes) | PartType::InlineBinary(bytes) => bytes.as_ref(),
        PartType::Text(text) | PartType::Html(text) => text.as_bytes(),
        PartType::Message(message) => message.raw_message.as_ref(),
        PartType::Multipart(_) => panic!("carried a multipart container"),
    };
    assert_eq!(carried, bytes);
}

#[test]
fn forward_skips_a_multipart_container_named_like_an_attachment() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path(), SOURCE_WITH_NAMED_CONTAINER);

    let composed = forward(&config, dir.path(), &[]);

    assert!(
        !composed.contains("weird"),
        "a multipart container named like an attachment was carried: {composed}"
    );
    assert!(
        !composed.contains("Content-Disposition: attachment"),
        "a multipart container was carried as an attachment: {composed}"
    );
    assert!(
        composed.contains("> the body inside the container"),
        "the quoted source body was dropped: {composed}"
    );
}
