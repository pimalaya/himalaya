//! # Raw command
//!
//! The clap argument the raw IMAP, SMTP and ManageSieve commands take
//! their input from.
//!
//! A command typed on the shell arrives with its backslash escapes
//! literal, so `\r` and `\n` are turned into real CRLF here and a whole
//! batch of commands can be passed inline.
//!
//! The trailing CRLF is each caller's policy: IMAP appends one when
//! missing, io-imap rejecting an unterminated command, where SMTP strips
//! it, io-smtp appending its own, and forbids batching.

use std::io::{IsTerminal, Read, stdin};

use anyhow::{Context, bail};
use clap::Parser;

use crate::shared::crlf;

/// Trailing positional resolving to a raw protocol command.
///
/// The positional wins, then piped stdin, read whole. Either way the
/// result goes through [`crlf::normalize`], so a bare `\n` becomes CRLF.
#[derive(Debug, Parser)]
pub struct RawCommandArg {
    /// The raw command lines, or the standard input when piped.
    ///
    /// A literal `\r` or `\n` in the argument becomes a real CRLF, so a
    /// batch of CRLF-separated commands can be passed inline.
    #[arg(name = "command-raw", value_name = "COMMAND", raw = true)]
    pub raw: Vec<String>,
}

impl RawCommandArg {
    /// Resolves the command, rejecting an empty result whatever its
    /// source.
    pub fn parse(&self) -> anyhow::Result<String> {
        let command = String::from_utf8(self.resolve()?).context("Command is not valid UTF-8")?;

        if command.trim().is_empty() {
            bail!("Command is empty");
        }

        Ok(command)
    }

    /// Reads the command off the positional or piped stdin, normalising
    /// its line endings to CRLF.
    fn resolve(&self) -> anyhow::Result<Vec<u8>> {
        if !self.raw.is_empty() {
            let command = self.raw.join(" ").replace("\\r", "").replace("\\n", "\r\n");
            return Ok(crlf::normalize(command.as_bytes()));
        }

        if !stdin().is_terminal() {
            let mut contents = Vec::new();
            stdin()
                .read_to_end(&mut contents)
                .context("Failed to read command from stdin")?;
            return Ok(crlf::normalize(&contents));
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
