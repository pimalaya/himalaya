//! Reusable clap arg for raw RFC 5322 message input.
//!
//! Ported verbatim from `mml::cli::args::MessageArg` so every
//! message-source command (shared `messages add`/`send`, per-protocol
//! `imap message save`, `maildir message save`, `jmap email import`,
//! `smtp message send`) accepts the same three forms: a file path, an
//! inline raw message, or stdin.

use std::{
    fs,
    io::{IsTerminal, stdin},
};

use anyhow::bail;
use clap::Parser;
use pimalaya_cli::clap::parsers::path_parser;

/// Trailing positional that resolves to a raw RFC 5322 message.
///
/// Resolution order:
///
/// 1. When the positional arg is non-empty: join the tokens with a
///    space, strip `\r` literals and turn `\n` literals into `\r\n`,
///    then treat the result as a path. If the path parses and the file
///    is readable, return its contents; otherwise treat the joined
///    value as the raw message verbatim.
/// 2. Otherwise, when stdin is piped, return stdin lines joined with
///    `\r\n`.
/// 3. Otherwise, bail.
#[derive(Debug, Parser)]
pub struct MessageArg {
    /// Can be a path to a file, raw message contents or nothing if
    /// piped via standard input.
    #[arg(name = "message-raw", value_name = "MESSAGE", raw = true)]
    pub raw: Vec<String>,
}

impl MessageArg {
    pub fn parse(&self) -> anyhow::Result<String> {
        if !self.raw.is_empty() {
            let mime = self.raw.join(" ").replace("\\r", "").replace("\\n", "\r\n");

            let Ok(path) = path_parser(&mime) else {
                return Ok(mime);
            };

            let Ok(mime) = fs::read_to_string(path) else {
                return Ok(mime);
            };

            return Ok(mime);
        }

        if !stdin().is_terminal() {
            let lines: Vec<_> = stdin().lines().map_while(Result::ok).collect();
            return Ok(lines.join("\r\n"));
        }

        bail!("Message cannot be empty");
    }
}
