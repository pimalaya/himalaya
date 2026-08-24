//! Reusable clap arg for raw protocol command input (IMAP/SMTP/ManageSieve).
//!
//! A raw command typed on the shell arrives with backslash escapes
//! left literal (e.g. `a0 NOOP\r\n` reaches the process as the bytes
//! `a0 NOOP\` `r` `\` `n`, not a real CRLF). This arg turns those
//! literal `\r`/`\n` escapes into real CRLF so a whole batch of
//! CRLF-separated commands can be passed inline, mirroring the
//! resolution `MessageArg` performs for raw messages.
//!
//! Callers apply their own trailing-CRLF policy: IMAP appends a final
//! CRLF when missing (io-imap rejects an unterminated command), while
//! SMTP strips it (io-smtp appends its own) and forbids batching.

use std::io::{IsTerminal, stdin};

use anyhow::bail;
use clap::Parser;

use crate::shared::crlf;

/// Trailing positional that resolves to a raw protocol command.
///
/// Resolution order:
///
/// 1. When the positional arg is non-empty: join the tokens with a
///    space, strip `\r` literals and turn `\n` literals into real
///    `\r\n`.
/// 2. Otherwise, when stdin is piped, return stdin lines joined with
///    `\r\n`.
/// 3. Otherwise, bail.
///
/// Whichever branch wins, the resolved bytes go through
/// [`crlf::normalize`](crate::shared::crlf::normalize) so any remaining
/// bare `\n` (e.g. from a real multi-line shell argument) becomes
/// canonical `\r\n`.
#[derive(Debug, Parser)]
pub struct RawCommandArg {
    /// The raw command line(s). Literal `\r`/`\n` in the argument are
    /// turned into real CRLF; can be piped via standard input instead.
    #[arg(name = "command-raw", value_name = "COMMAND", raw = true)]
    pub raw: Vec<String>,
}

impl RawCommandArg {
    /// Resolves the command and rejects an empty result, whatever the
    /// source.
    pub fn parse(&self) -> anyhow::Result<String> {
        let command = self.resolve()?;

        if command.trim().is_empty() {
            bail!("Command is empty");
        }

        Ok(command)
    }

    /// Resolves the raw command from the positional arg or piped stdin
    /// (see the type docs), normalising line endings to CRLF.
    fn resolve(&self) -> anyhow::Result<String> {
        if !self.raw.is_empty() {
            let command = self.raw.join(" ").replace("\\r", "").replace("\\n", "\r\n");
            return Ok(crlf::normalize(&command));
        }

        if !stdin().is_terminal() {
            let lines: Vec<_> = stdin().lines().map_while(Result::ok).collect();
            return Ok(lines.join("\r\n"));
        }

        bail!("No command provided: pass it as an argument or via stdin");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn literal_escapes_become_real_crlf() {
        // NOTE: mirrors what `resolve` does to a shell-escaped batch.
        let command = "a0 CAPABILITY\\r\\na1 NOOP\\r\\n"
            .replace("\\r", "")
            .replace("\\n", "\r\n");
        assert_eq!(command, "a0 CAPABILITY\r\na1 NOOP\r\n");
    }
}
