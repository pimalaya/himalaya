//! # ManageSieve raw
//!
//! The `sieve raw` command, a byte-for-byte passthrough to the server.

use anyhow::{Result, bail};
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::{Message, Printer};

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

        printer.out(Message::new(response.to_string()))
    }
}
