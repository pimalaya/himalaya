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
/// [`normalize_crlf`] so IMAP `APPEND` (which rejects bare newlines)
/// and the other line-oriented backends receive canonical `\r\n`
/// endings regardless of the source's convention.
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

            // Treat the value as a file only when it actually points at
            // one; otherwise it is the raw inline message. A real file
            // that fails to read (e.g. non-UTF-8 bytes) is a hard error,
            // never silently reinterpreted as the message body.
            if let Some(path) = path_parser(&mime).ok().filter(|path| path.is_file()) {
                let contents = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read message file `{}`", path.display()))?;
                return Ok(normalize_crlf(&contents));
            }

            return Ok(normalize_crlf(&mime));
        }

        if !stdin().is_terminal() {
            let lines: Vec<_> = stdin().lines().map_while(Result::ok).collect();
            return Ok(lines.join("\r\n"));
        }

        bail!("Message cannot be empty");
    }
}

/// Rewrites bare line feeds to CRLF, idempotently: a `\n` already
/// preceded by `\r` is left untouched, every other `\n` gets a `\r`
/// inserted before it. A message read from a Unix-LF file or passed
/// inline with real newlines is thus made RFC 5322 / IMAP `APPEND`
/// compliant without corrupting content that is already CRLF.
fn normalize_crlf(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev = '\0';

    for ch in input.chars() {
        if ch == '\n' && prev != '\r' {
            out.push('\r');
        }
        out.push(ch);
        prev = ch;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::normalize_crlf;

    #[test]
    fn bare_lf_gains_cr() {
        assert_eq!(normalize_crlf("a\nb\n"), "a\r\nb\r\n");
    }

    #[test]
    fn existing_crlf_is_untouched() {
        assert_eq!(normalize_crlf("a\r\nb\r\n"), "a\r\nb\r\n");
    }

    #[test]
    fn mixed_endings_converge_to_crlf() {
        assert_eq!(normalize_crlf("a\r\nb\nc"), "a\r\nb\r\nc");
    }

    #[test]
    fn lone_cr_is_preserved() {
        assert_eq!(normalize_crlf("a\rb"), "a\rb");
    }
}
