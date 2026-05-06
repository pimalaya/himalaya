use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::jmap::{client::JmapClient, thread::get::JmapThreadGetCommand};

/// Manage JMAP threads.
#[derive(Debug, Subcommand)]
pub enum JmapThreadCommand {
    /// Fetch threads by ID (Thread/get).
    Get(JmapThreadGetCommand),
}

impl JmapThreadCommand {
    pub fn execute(self, printer: &mut impl Printer, client: JmapClient) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
        }
    }
}
