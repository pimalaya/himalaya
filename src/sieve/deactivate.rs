use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::sieve::client::SieveClient;

/// Disable the currently active server-side Sieve script.
#[derive(Debug, Parser)]
pub struct SieveScriptDeactivateCommand;

impl SieveScriptDeactivateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        client.set_active(None)?;
        printer.out(Message::new("Sieve script successfully deactivated"))
    }
}
