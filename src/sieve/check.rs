use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::sieve::{client::SieveClient, script::SieveScriptArg};

/// Validate a Sieve script on the server without storing it.
#[derive(Debug, Parser)]
pub struct SieveScriptCheckCommand {
    #[command(flatten)]
    pub script: SieveScriptArg,
}

impl SieveScriptCheckCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        client.check_script(&self.script.read()?)?;
        printer.out(Message::new("Sieve script is valid"))
    }
}
