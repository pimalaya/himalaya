use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::m2dir::{arg::M2dirNameArg, client::M2dirClient};

/// Create the given m2dir folder.
///
/// Initialises the m2store at the client root if needed, then creates
/// the m2dir folder named after `name` (relative to the store root).
#[derive(Debug, Parser)]
pub struct M2dirMailboxCreateCommand {
    #[command(flatten)]
    pub m2dir_name: M2dirNameArg,
}

impl M2dirMailboxCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        client.init_store()?;
        client.create_m2dir(&self.m2dir_name.inner)?;
        printer.out(Message::new("m2dir folder successfully created"))
    }
}
