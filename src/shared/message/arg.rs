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

use crate::{error::ErrorCode, mcp, shared::crlf};

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
    /// It comes last, every word after it taken as part of it: put it
    /// after `--` when it opens on a hyphen.
    #[arg(name = "message-raw", value_name = "MESSAGE")]
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
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
    fn resolve(&self) -> anyhow::Result<Vec<u8>> {
        if !self.raw.is_empty() {
            let mime = self.raw.join(" ").replace("\\r", "").replace("\\n", "\r\n");

            // NOTE: a real file that fails to read is a hard error, never
            // silently reinterpreted as the message body.
            // NOTE: an MCP caller names no local file: what it gives is
            // the message, whatever path it happens to spell.
            let path = path_parser(&mime)
                .ok()
                .filter(|path| !mcp::paths_closed() && path.is_file());

            if let Some(path) = path {
                let contents = fs::read(&path)
                    .with_context(|| format!("Failed to read message file `{}`", path.display()))?;
                return Ok(crlf::normalize(&contents));
            }

            // NOTE: a mistyped path would otherwise be stored as a message
            // made of that path alone.
            if !opens_on_a_field(&mime) {
                if mcp::paths_closed() {
                    bail!("The message opens on no header field");
                }

                bail!("No file at `{mime}`, nor a header field opening a message");
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

/// Whether an inline message opens on a header field, a name of printable
/// ASCII then a colon (RFC 5322 §2.2).
fn opens_on_a_field(message: &str) -> bool {
    let Some((name, _)) = message.split_once(':') else {
        return false;
    };

    !name.is_empty() && name.bytes().all(|b| b.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use crate::shared::message::arg::MessageArg;

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
}
