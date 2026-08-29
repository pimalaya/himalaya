//! # Microsoft Graph attachments command
//!
//! The `msgraph attachments` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    msgraph::{
        attachments::{
            create::MsgraphAttachmentCreateCommand, delete::MsgraphAttachmentDeleteCommand,
            get::MsgraphAttachmentGetCommand, list::MsgraphAttachmentListCommand,
        },
        client::MsgraphClient,
    },
};

/// Manage Microsoft Graph message attachments
/// (`me.messages.attachments`).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MsgraphAttachmentsCommand {
    List(MsgraphAttachmentListCommand),
    Get(MsgraphAttachmentGetCommand),
    Create(MsgraphAttachmentCreateCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(MsgraphAttachmentDeleteCommand),
}

impl MsgraphAttachmentsCommand {
    /// Runs the subcommand against the account's Graph client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}
