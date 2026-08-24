use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::{shared::raw::RawCommandArg, sieve::client::SieveClient};

/// Send one raw ManageSieve command and print its complete response.
///
/// Raw is intended for diagnostics and commands not yet modelled by the
/// high-level API. It accepts one command line; use `put` or `check` for
/// literal-bearing commands so response framing stays synchronized.
#[derive(Debug, Parser)]
pub struct SieveRawCommand {
    #[command(flatten)]
    pub command: RawCommandArg,
}

impl SieveRawCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        printer.out(Message::new(client.raw(&self.command.parse()?)?))
    }
}
