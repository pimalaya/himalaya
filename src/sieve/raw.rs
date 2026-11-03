//! # ManageSieve raw
//!
//! The `sieve raw` command, a byte-for-byte passthrough to the server.

use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use io_managesieve::{client::ManagesieveClient as _, rfc5804::response::ManagesieveStatus};
use pimalaya_cli::{error::PartialOutcome, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{shared::raw::RawCommandArg, sieve::client::SieveClient};

/// Send a raw ManageSieve command and print the verbatim server response.
///
/// One line goes out without its trailing CRLF, which io-managesieve
/// appends before reading the whole response back, literals included. A NO
/// or a BYE comes back as output rather than as an error.
///
/// The exchange reads exactly one response, so batching is refused, and a
/// literal-bearing command such as `PUTSCRIPT` has its own subcommand.
#[derive(Debug, Parser)]
pub struct SieveRawCommand {
    #[command(flatten)]
    pub command: RawCommandArg,
}

impl SieveRawCommand {
    /// Sends the command and prints the raw response.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        // NOTE: io-managesieve appends the trailing CRLF itself, so the one
        // the caller may have added is stripped.
        let command = self.command.parse()?;
        let command = command.trim_end_matches(['\r', '\n']);

        // NOTE: an interior newline would be a second command the
        // response reader never accounts for, desyncing the stream.
        if command.contains('\n') {
            bail!("ManageSieve raw accepts a single command line; batching is not supported");
        }

        let response = client.raw(command.into())?;

        for line in &response.data {
            printer.event(SieveRawEvent::Data {
                text: line.to_string(),
            })?;
        }

        let completion = response.completion;
        let refused = completion.status != ManagesieveStatus::Ok;

        printer.out(SieveRawOutput {
            status: completion.status.as_str(),
            code: completion.code.map(|code| code.to_string()),
            text: completion.text,
        })?;

        if refused {
            return Err(PartialOutcome::default().into());
        }

        Ok(())
    }
}

/// What `sieve raw` prints before its result: each data line of the
/// response.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub(crate) enum SieveRawEvent {
    /// A data line, its literals rendered as quoted strings.
    Data {
        /// The line.
        text: String,
    },
}

impl fmt::Display for SieveRawEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data { text } => writeln!(f, "{text}"),
        }
    }
}

/// The result of `sieve raw`: the completion line, split into its parts.
/// A `NO` or `BYE` exits 2, the server having answered.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SieveRawOutput {
    /// `OK`, `NO` or `BYE`.
    status: &'static str,
    /// The response code, `null` when there is none.
    code: Option<String>,
    /// The human-readable text, `null` when there is none.
    text: Option<String>,
}

impl fmt::Display for SieveRawOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.status)?;

        if let Some(code) = &self.code {
            write!(f, " ({code})")?;
        }
        if let Some(text) = &self.text {
            write!(f, " {text}")?;
        }

        writeln!(f)
    }
}
