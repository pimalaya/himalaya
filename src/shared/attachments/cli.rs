use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::shared::{
    attachments::{download::AttachmentDownloadCommand, list::AttachmentListCommand},
    client::EmailClient,
};

/// Shared API to manage attachments for the active account.
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
    pub fn execute(self, printer: &mut impl Printer, client: EmailClient) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, client),
            Self::Download(cmd) => cmd.execute(printer, client),
        }
    }
}
