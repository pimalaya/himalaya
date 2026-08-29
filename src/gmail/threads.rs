//! # Gmail threads
//!
//! The `gmail threads` command family, covering `users.threads`.

pub mod delete;
pub mod get;
pub mod list;
pub mod modify;
pub mod trash;
pub mod untrash;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        threads::{
            delete::GmailThreadDeleteCommand, get::GmailThreadGetCommand,
            list::GmailThreadsListCommand, modify::GmailThreadModifyCommand,
            trash::GmailThreadTrashCommand, untrash::GmailThreadUntrashCommand,
        },
    },
};

/// Manage Gmail threads (users.threads).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailThreadsCommand {
    List(GmailThreadsListCommand),
    Get(GmailThreadGetCommand),
    Modify(GmailThreadModifyCommand),
    Trash(GmailThreadTrashCommand),
    Untrash(GmailThreadUntrashCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailThreadDeleteCommand),
}

impl GmailThreadsCommand {
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
            Self::Modify(cmd) => cmd.execute(printer, client),
            Self::Trash(cmd) => cmd.execute(printer, client),
            Self::Untrash(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}
