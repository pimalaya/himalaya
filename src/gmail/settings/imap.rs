pub mod get;
pub mod set;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::imap::{get::GmailSettingsImapGetCommand, set::GmailSettingsImapSetCommand},
    },
};

/// Manage the Gmail IMAP access settings
/// (users.settings.getImap / updateImap).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsImapCommand {
    Get(GmailSettingsImapGetCommand),
    #[command(visible_aliases = ["update"])]
    Set(GmailSettingsImapSetCommand),
}

impl GmailSettingsImapCommand {
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
