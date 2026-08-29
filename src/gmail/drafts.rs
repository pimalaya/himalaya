//! # Gmail drafts
//!
//! The `gmail drafts` command family, covering `users.drafts`.

pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod send;
pub mod update;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        drafts::{
            create::GmailDraftCreateCommand, delete::GmailDraftDeleteCommand,
            get::GmailDraftGetCommand, list::GmailDraftsListCommand, send::GmailDraftSendCommand,
            update::GmailDraftUpdateCommand,
        },
    },
};

/// Manage Gmail drafts (users.drafts).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailDraftsCommand {
    List(GmailDraftsListCommand),
    Get(GmailDraftGetCommand),
    Create(GmailDraftCreateCommand),
    Update(GmailDraftUpdateCommand),
    Send(GmailDraftSendCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailDraftDeleteCommand),
}

impl GmailDraftsCommand {
    /// Runs the subcommand against the account's Gmail client.
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
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Send(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}
