//! # m2dir create
//!
//! The `m2dir create` command, laying out a new folder under the store
//! root.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::m2dir::{arg::M2dirNameArg, client::M2dirClient};

/// Create the given m2dir folder.
///
/// The store is initialised at the client root first when it does not
/// exist yet.
#[derive(Debug, Parser)]
pub struct M2dirMailboxCreateCommand {
    #[command(flatten)]
    pub m2dir_name: M2dirNameArg,
}

impl M2dirMailboxCreateCommand {
    /// Initialises the store if needed, then creates the folder.
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        client.init_store()?;
        client.create_m2dir(&self.m2dir_name.inner)?;
        printer.out(Message::new("m2dir folder successfully created"))
    }
}
