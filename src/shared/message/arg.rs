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

use anyhow::{Context, bail};
use clap::Parser;
use pimalaya_cli::clap::parsers::path_parser;

use crate::shared::crlf;

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
///
/// Whichever branch wins, the resolved bytes go through
/// [`crlf::normalize`](crate::shared::crlf::normalize) so IMAP `APPEND`
/// (which rejects bare newlines) and the other line-oriented backends
/// receive canonical `\r\n` endings regardless of the source's
/// convention.
#[derive(Debug, Parser)]
pub struct MessageArg {
    /// Can be a path to a file, raw message contents or nothing if
    /// piped via standard input.
    #[arg(name = "message-raw", value_name = "MESSAGE", raw = true)]
    pub raw: Vec<String>,
}

impl MessageArg {
    pub fn parse(&self) -> anyhow::Result<String> {
        let message = self.resolve()?;

        // Reject an empty message uniformly, whatever the source: an
        // empty positional (`-- ''`), an empty file, or empty stdin
        // would otherwise reach the backend and fail with an opaque
        // server error (e.g. IMAP `APPEND … Zero-length message`).
        if message.trim().is_empty() {
            bail!("Message is empty");
        }

        Ok(message)
    }

    /// Resolves the raw message from the positional arg, a file path,
    /// or piped stdin (see the type docs), normalising line endings to
    /// CRLF. Emptiness is rejected by [`Self::parse`].
    fn resolve(&self) -> anyhow::Result<String> {
        if !self.raw.is_empty() {
            let mime = self.raw.join(" ").replace("\\r", "").replace("\\n", "\r\n");

            // Treat the value as a file only when it actually points at
            // one; otherwise it is the raw inline message. A real file
            // that fails to read (e.g. non-UTF-8 bytes) is a hard error,
            // never silently reinterpreted as the message body.
            if let Some(path) = path_parser(&mime).ok().filter(|path| path.is_file()) {
                let contents = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read message file `{}`", path.display()))?;
                return Ok(crlf::normalize(&contents));
            }

            return Ok(crlf::normalize(&mime));
        }

        if !stdin().is_terminal() {
            let lines: Vec<_> = stdin().lines().map_while(Result::ok).collect();
            return Ok(lines.join("\r\n"));
        }

        bail!("No message provided: pass it as an argument, a file path, or via stdin");
    }
}
