pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod update;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        labels::{
            create::GmailLabelCreateCommand, delete::GmailLabelDeleteCommand,
            get::GmailLabelGetCommand, list::GmailLabelsListCommand,
            update::GmailLabelUpdateCommand,
        },
    },
};

/// Manage Gmail labels (users.labels).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailLabelsCommand {
    List(GmailLabelsListCommand),
    Get(GmailLabelGetCommand),
    Create(GmailLabelCreateCommand),
    Update(GmailLabelUpdateCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailLabelDeleteCommand),
}

impl GmailLabelsCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}
