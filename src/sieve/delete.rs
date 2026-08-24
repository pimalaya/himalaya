use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::sieve::client::SieveClient;

/// Delete one server-side Sieve script.
#[derive(Debug, Parser)]
pub struct SieveScriptDeleteCommand {
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl SieveScriptDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        client.delete_script(&self.name)?;
        printer.out(Message::new("Sieve script successfully deleted"))
    }
}
