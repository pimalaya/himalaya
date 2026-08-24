use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::sieve::client::SieveClient;

/// Make one server-side Sieve script active.
#[derive(Debug, Parser)]
pub struct SieveScriptActivateCommand {
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl SieveScriptActivateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        client.set_active(Some(&self.name))?;
        printer.out(Message::new("Sieve script successfully activated"))
    }
}
