//! # Message argument
//!
//! The clap argument every command taking a raw RFC 5322 message reads
//! its input from.
//!
//! Ported from mml so the shared commands and the protocol-specific save
//! and send ones all accept the same three forms: a file path, an inline
//! message, or piped standard input.

use std::{
    fs,
    io::{IsTerminal, Read, stdin},
};

use anyhow::{Context, bail};
use clap::Parser;
use pimalaya_cli::clap::parsers::path_parser;

use crate::shared::crlf;

/// Trailing positional resolving to a raw RFC 5322 message.
///
/// The positional wins, read as a file when it points at one and as the
/// message itself otherwise, then piped stdin. Either way the result goes
/// through [`crlf::normalize`], IMAP `APPEND` rejecting bare newlines.
#[derive(Debug, Parser)]
pub struct MessageArg {
    /// A path to a file, the message itself, or the standard input when
    /// piped.
    #[arg(name = "message-raw", value_name = "MESSAGE", raw = true)]
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
            if let Some(path) = path_parser(&mime).ok().filter(|path| path.is_file()) {
                let contents = fs::read(&path)
                    .with_context(|| format!("Failed to read message file `{}`", path.display()))?;
                return Ok(crlf::normalize(&contents));
            }

            return Ok(crlf::normalize(mime.as_bytes()));
        }

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
