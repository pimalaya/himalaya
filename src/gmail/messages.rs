pub mod batch_delete;
pub mod batch_modify;
pub mod delete;
pub mod get;
pub mod import;
pub mod insert;
pub mod list;
pub mod modify;
pub mod send;
pub mod trash;
pub mod untrash;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        messages::{
            batch_delete::GmailMessageBatchDeleteCommand,
            batch_modify::GmailMessageBatchModifyCommand, delete::GmailMessageDeleteCommand,
            get::GmailMessageGetCommand, import::GmailMessageImportCommand,
            insert::GmailMessageInsertCommand, list::GmailMessagesListCommand,
            modify::GmailMessageModifyCommand, send::GmailMessageSendCommand,
            trash::GmailMessageTrashCommand, untrash::GmailMessageUntrashCommand,
        },
    },
};

/// Manage Gmail messages (users.messages).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailMessagesCommand {
    List(GmailMessagesListCommand),
    Get(GmailMessageGetCommand),
    Send(GmailMessageSendCommand),
    Import(GmailMessageImportCommand),
    Insert(GmailMessageInsertCommand),
    Modify(GmailMessageModifyCommand),
    Trash(GmailMessageTrashCommand),
    Untrash(GmailMessageUntrashCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailMessageDeleteCommand),
    BatchModify(GmailMessageBatchModifyCommand),
    BatchDelete(GmailMessageBatchDeleteCommand),
}

impl GmailMessagesCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Send(cmd) => cmd.execute(printer, client),
            Self::Import(cmd) => cmd.execute(printer, client),
            Self::Insert(cmd) => cmd.execute(printer, client),
            Self::Modify(cmd) => cmd.execute(printer, client),
            Self::Trash(cmd) => cmd.execute(printer, client),
            Self::Untrash(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::BatchModify(cmd) => cmd.execute(printer, client),
            Self::BatchDelete(cmd) => cmd.execute(printer, client),
        }
    }
}
