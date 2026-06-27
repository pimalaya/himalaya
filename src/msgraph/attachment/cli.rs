use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::msgraph::{
    attachment::{
        create::MsgraphAttachmentCreateCommand, delete::MsgraphAttachmentDeleteCommand,
        get::MsgraphAttachmentGetCommand, list::MsgraphAttachmentListCommand,
    },
    client::MsgraphClient,
};

/// Manage Microsoft Graph message attachments
/// (`me.messages.attachments`).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MsgraphAttachmentCommand {
    List(MsgraphAttachmentListCommand),
    Get(MsgraphAttachmentGetCommand),
    Create(MsgraphAttachmentCreateCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(MsgraphAttachmentDeleteCommand),
}

impl MsgraphAttachmentCommand {
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
