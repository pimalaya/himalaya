pub mod get;
pub mod set;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::pop::{get::GmailSettingsPopGetCommand, set::GmailSettingsPopSetCommand},
    },
};

/// Manage the Gmail POP access settings
/// (users.settings.getPop / updatePop).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsPopCommand {
    Get(GmailSettingsPopGetCommand),
    #[command(visible_aliases = ["update"])]
    Set(GmailSettingsPopSetCommand),
}

impl GmailSettingsPopCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Set(cmd) => cmd.execute(printer, client),
        }
    }
}
