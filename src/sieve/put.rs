use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::sieve::{client::SieveClient, script::SieveScriptArg};

/// Validate capacity and upload one server-side Sieve script.
#[derive(Debug, Parser)]
pub struct SieveScriptPutCommand {
    /// The script name.
    #[arg(value_name = "NAME")]
    pub name: String,
    #[command(flatten)]
    pub script: SieveScriptArg,
}

impl SieveScriptPutCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        client.put_script(&self.name, &self.script.read()?)?;
        printer.out(Message::new("Sieve script successfully uploaded"))
    }
}
