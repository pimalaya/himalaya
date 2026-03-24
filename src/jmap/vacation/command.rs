use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{
    account::JmapAccount,
    vacation::{get::GetVacationCommand, set::SetVacationCommand},
};

/// Manage JMAP vacation response.
#[derive(Debug, Subcommand)]
pub enum VacationCommand {
    /// Get the vacation response (VacationResponse/get).
    Get(GetVacationCommand),
    /// Update the vacation response (VacationResponse/set).
    Set(SetVacationCommand),
}

impl VacationCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account),
            Self::Set(cmd) => cmd.execute(printer, account),
        }
    }
}
