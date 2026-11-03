//! # SMTP raw
//!
//! The `smtp raw` command, a byte-for-byte passthrough to the server.

use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use io_smtp::client::SmtpClient as _;
use pimalaya_cli::{error::PartialOutcome, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{shared::raw::RawCommandArg, smtp::client::SmtpClient};

/// Send a raw SMTP command and print the verbatim server reply.
///
/// One line goes out without its trailing CRLF, which io-smtp appends
/// before reading the full reply back. Any reply code comes back as output
/// rather than as an error.
///
/// The exchange reads exactly one reply, so batching is refused, and
/// `DATA` and `STARTTLS`, which switch the stream into another mode, are
/// not supported.
#[derive(Debug, Parser)]
pub struct SmtpRawCommand {
    #[command(flatten)]
    pub command: RawCommandArg,
}

impl SmtpRawCommand {
    /// Sends the command and prints the raw reply.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        // NOTE: io-smtp appends the trailing CRLF itself, so the one the
        // caller may have added is stripped.
        let command = self.command.parse()?;
        let command = command.trim_end_matches(['\r', '\n']);

        // NOTE: an interior newline would be a second command the reply
        // parser never accounts for, desyncing the stream.
        if command.contains('\n') {
            bail!("SMTP raw accepts a single command line; batching is not supported");
        }

        let reply = client.raw(command.to_string().into())?;
        let output = SmtpRawOutput::parse(&reply);
        let refused = output.code.is_some_and(|code| code >= 400);

        printer.out(output)?;

        if refused {
            return Err(PartialOutcome::default().into());
        }

        Ok(())
    }
}

/// The result of `smtp raw`: the reply, split into its code and text. A
/// 4xx or 5xx reply exits 2, the server having answered.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SmtpRawOutput {
    /// The reply code, `null` when the reply does not start with one.
    code: Option<u16>,
    /// The text of the reply, one line per line, the code and its
    /// separator stripped when parsed.
    text: String,
}

impl SmtpRawOutput {
    /// Splits a reply into its code and its text lines.
    fn parse(reply: &str) -> Self {
        let lines: Vec<&str> = reply.lines().filter(|line| !line.is_empty()).collect();

        let code = lines.first().and_then(|line| line.get(..3)?.parse().ok());

        let Some(code) = code else {
            return Self {
                code: None,
                text: reply.trim_end().to_owned(),
            };
        };

        let text: Vec<&str> = lines
            .iter()
            .map(|line| line.get(4..).unwrap_or_default())
            .collect();

        Self {
            code: Some(code),
            text: text.join("\n"),
        }
    }
}

impl fmt::Display for SmtpRawOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.code {
            Some(code) => writeln!(f, "{code} {}", self.text),
            None => writeln!(f, "{}", self.text),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::smtp::raw::SmtpRawOutput;

    #[test]
    fn a_multiline_reply_keeps_its_code_and_lines() {
        let output = SmtpRawOutput::parse("250-mx.example.org\r\n250 SIZE 1000\r\n");
        assert_eq!(
            serde_json::to_value(output).unwrap(),
            json!({ "code": 250, "text": "mx.example.org\nSIZE 1000" })
        );

        let output = SmtpRawOutput::parse("garbage\r\n");
        assert_eq!(
            serde_json::to_value(output).unwrap(),
            json!({ "code": null, "text": "garbage" })
        );
    }
}
