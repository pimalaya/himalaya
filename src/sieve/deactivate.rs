use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::sieve::client::SieveClient;

/// Disable the currently active server-side Sieve script.
///
/// Filtering stops until a script is activated again; doing it twice is
/// not an error.
#[derive(Debug, Parser)]
pub struct SieveScriptDeactivateCommand;

impl SieveScriptDeactivateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        client.activate_script(None)?;
        printer.out(Message::new("Sieve script successfully deactivated"))
    }
}
