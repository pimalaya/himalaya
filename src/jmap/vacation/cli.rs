use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::jmap::{
    account::JmapAccount,
    vacation::{get::JmapVacationGetCommand, set::JmapVacationSetCommand},
};

/// Manage JMAP vacation response.
#[derive(Debug, Subcommand)]
pub enum JmapVacationCommand {
    /// Get the vacation response (VacationResponse/get).
    Get(JmapVacationGetCommand),
    /// Update the vacation response (VacationResponse/set).
    Set(JmapVacationSetCommand),
}

impl JmapVacationCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account),
            Self::Set(cmd) => cmd.execute(printer, account),
        }
    }
}
