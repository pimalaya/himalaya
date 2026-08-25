use anyhow::{Result, bail};
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::{shared::raw::RawCommandArg, sieve::client::SieveClient};

/// Send a raw ManageSieve command and print the verbatim server
/// response.
///
/// The command is a single line sent without trailing CRLF (e.g.
/// `CAPABILITY`, `LISTSCRIPTS`, `GETSCRIPT "main"`); io-managesieve
/// appends the CRLF and reads the whole response back, literals
/// included. A NO or BYE is returned as output, not as an error.
/// Batching is rejected and a literal-bearing command such as
/// `PUTSCRIPT` has its own subcommand, the exchange reading exactly one
/// response.
#[derive(Debug, Parser)]
pub struct SieveRawCommand {
    #[command(flatten)]
    pub command: RawCommandArg,
}

impl SieveRawCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        // NOTE: io-managesieve appends the trailing CRLF itself, so
        // strip the one the caller may have added (literal or real).
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
