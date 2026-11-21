//! # Message argument
//!
//! The clap argument every command taking a raw RFC 5322 message reads
//! its input from.
//!
//! Ported from mml so the unified commands and the protocol-specific save
//! and send ones all accept the same three forms: a file path, an inline
//! message, or piped standard input.

use std::{
    fs,
    io::{IsTerminal, Read, stdin},
};

use anyhow::{Context, bail};
use clap::Parser;
use pimalaya_cli::{clap::parsers::path_parser, session::check_stdin};

use crate::{crlf, error::ErrorCode, mcp};

/// Trailing positional resolving to a raw RFC 5322 message.
///
/// The positional wins, read as a file when it points at one and as the
/// message itself otherwise, then piped stdin. Either way the result goes
/// through [`crlf::normalize`], IMAP `APPEND` rejecting bare newlines.
#[derive(Debug, Parser)]
pub struct MessageArg {
    /// A path to a file, the message itself, or the standard input when
    /// piped.
    ///
    /// Options may come before or after it. Put it after `--` when a word
    /// of it opens on a hyphen.
    #[arg(name = "message-raw", value_name = "MESSAGE")]
    pub raw: Vec<String>,
}

impl MessageArg {
    /// Resolves the message, rejecting an empty result whatever its
    /// source.
    ///
    /// The message is kept as bytes: RFC 5322 allows 8-bit content in
    /// any charset, which UTF-8 decoding would reject.
    pub fn parse(&self) -> anyhow::Result<Vec<u8>> {
        let message = self.resolve()?;

        // NOTE: an empty message would otherwise reach the backend and
        // fail there with an opaque server error.
        if message.trim_ascii().is_empty() {
            bail!("Message is empty");
        }

        Ok(message)
    }

    /// Reads the message off the positional, a file it names, or piped
    /// stdin, normalising its line endings to CRLF.
    ///
    /// The path is looked up as typed, before the `\n` and `\r` escapes of
    /// an inline message are decoded; a file that fails to read is an
    /// error, never read as the message. An MCP caller names no local file:
    /// what it gives is the message, whatever path it spells.
    fn resolve(&self) -> anyhow::Result<Vec<u8>> {
        if !self.raw.is_empty() {
            let joined = self.raw.join(" ");
            let mime = joined.replace("\\r", "").replace("\\n", "\r\n");

            let path = path_parser(&joined)
                .ok()
                .filter(|path| !mcp::paths_closed() && path.is_file());

            if let Some(path) = path {
                let contents = fs::read(&path)
                    .with_context(|| format!("Failed to read message file `{}`", path.display()))?;
                return Ok(crlf::normalize(&contents));
            }

            if !opens_on_a_field(&mime) {
                if mcp::paths_closed() {
                    bail!("The message opens on no header field");
                }

                bail!("No file at `{joined}`, nor a header field opening a message");
            }

            return Ok(crlf::normalize(mime.as_bytes()));
        }

        check_stdin(ErrorCode::StdinTaken)?;

        if !stdin().is_terminal() {
            let mut contents = Vec::new();
            stdin()
                .read_to_end(&mut contents)
                .context("Failed to read message from stdin")?;
            return Ok(crlf::normalize(&contents));
        }

        bail!("No message provided: pass it as an argument, a file path, or via stdin");
    }
}

/// Whether an inline message opens on a header field, a name then a colon
/// (RFC 5322 §2.2), so a mistyped path is not stored as a message made of
/// itself.
///
/// The name is held to the letters, digits, `-`, `_` and `.` header names
/// are spelled with, which a path holding a colon (a Maildir file name, its
/// `:2,` info) breaks with a separator, and a one-letter name followed by a
/// separator is a Windows drive.
fn opens_on_a_field(message: &str) -> bool {
    let Some((name, value)) = message.split_once(':') else {
        return false;
    };

    let named = name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
    let drive = name.len() == 1 && value.starts_with(['/', '\\']);

    !name.is_empty() && named && !drive
}

#[cfg(test)]
mod tests {
    use crate::unified::message::arg::MessageArg;

    fn arg(raw: &str) -> MessageArg {
        MessageArg {
            raw: vec![String::from(raw)],
        }
    }

    /// A value naming no file is a message only when it opens on a header
    /// field, so a mistyped path is refused instead of stored.
    #[test]
    fn a_missing_path_is_not_a_message() {
        let err = arg("/nonexistent/file.eml").parse().unwrap_err();
        assert!(err.to_string().contains("No file at"), "{err}");

        let message = arg("Subject: hi\\n\\nbody").parse().unwrap();
        assert_eq!(message, b"Subject: hi\r\n\r\nbody");
    }

    /// A Maildir file name holds a colon, as a Windows drive does, so a
    /// mistyped path holding one opens on no header field either.
    #[test]
    fn a_missing_path_holding_a_colon_is_not_a_message() {
        for path in [
            "/nonexistent/Mail/cur/1700000000.host:2,S",
            "nonexistent/cur/1700000000.host:2,S",
            "C:\\Users\\nick\\mail.eml",
            "C:/Users/nick/mail.eml",
        ] {
            let err = arg(path).parse().unwrap_err();
            assert!(err.to_string().contains("No file at"), "{path}: {err}");
        }

        let message = arg("X-Spam.Level: *\\n\\nbody").parse().unwrap();
        assert_eq!(message, b"X-Spam.Level: *\r\n\r\nbody");
    }

    /// The path is looked up as typed, before the `\n` and `\r` escapes of
    /// an inline message are decoded.
    #[test]
    fn a_path_holding_a_backslash_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("a\\nb");
        std::fs::create_dir(&sub).unwrap();
        let path = sub.join("m.eml");
        std::fs::write(&path, "Subject: hi\n\nbody\n").unwrap();

        let message = arg(path.to_str().unwrap()).parse().unwrap();
        assert_eq!(message, b"Subject: hi\r\n\r\nbody\r\n");
    }
}
