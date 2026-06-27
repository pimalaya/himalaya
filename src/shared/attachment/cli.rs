use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::shared::{
    attachment::{download::AttachmentDownloadCommand, list::AttachmentListCommand},
    client::EmailClient,
};

/// Manage attachments using the shared API.
///
/// An attachment is a binary part of a message.
#[derive(Debug, Subcommand)]
pub enum AttachmentCommand {
    #[command(visible_alias = "ls")]
    List(AttachmentListCommand),
    #[command(visible_alias = "dl")]
    Download(AttachmentDownloadCommand),
}

impl AttachmentCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Download(cmd) => cmd.execute(printer, account, client),
        }
    }
}
