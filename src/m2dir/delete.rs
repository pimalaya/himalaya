use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::m2dir::{arg::M2dirNameArg, client::M2dirClient};

/// Delete the given m2dir folder.
#[derive(Debug, Parser)]
pub struct M2dirMailboxDeleteCommand {
    #[command(flatten)]
    pub m2dir_name: M2dirNameArg,
}

impl M2dirMailboxDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir_name.inner)?;
        client.delete_m2dir(path)?;
        printer.out(Message::new("m2dir folder successfully deleted"))
    }
}
