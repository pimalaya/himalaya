pub mod create;
pub mod delete;
pub mod get;
pub mod list;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::delegates::{
            create::GmailSettingsDelegateCreateCommand, delete::GmailSettingsDelegateDeleteCommand,
            get::GmailSettingsDelegateGetCommand, list::GmailSettingsDelegatesListCommand,
        },
    },
};

/// Manage Gmail delegates (users.settings.delegates).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsDelegatesCommand {
    List(GmailSettingsDelegatesListCommand),
    Get(GmailSettingsDelegateGetCommand),
    Create(GmailSettingsDelegateCreateCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailSettingsDelegateDeleteCommand),
}

impl GmailSettingsDelegatesCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}
