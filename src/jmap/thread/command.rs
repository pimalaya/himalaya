use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{account::JmapAccount, thread::get::GetThreadCommand};

/// Manage JMAP threads.
#[derive(Debug, Subcommand)]
pub enum ThreadCommand {
    /// Fetch threads by ID (Thread/get).
    Get(GetThreadCommand),
}

impl ThreadCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account),
        }
    }
}
