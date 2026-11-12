//! # Attachment command
//!
//! The `attachment` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    capability,
    shared::{
        attachment::{download::AttachmentDownloadCommand, list::AttachmentListCommand},
        client::EmailClient,
    },
};

/// Manage attachments using the unified API.
///
/// An attachment is a binary part of a message.
#[derive(Debug, Subcommand)]
pub enum AttachmentCommand {
    #[command(visible_alias = "ls", after_long_help = capability::help("attachment", "list"))]
    List(AttachmentListCommand),
    #[command(visible_alias = "dl", after_long_help = capability::help("attachment", "download"))]
    Download(AttachmentDownloadCommand),
}

impl AttachmentCommand {
    /// Runs the subcommand against the active account's client.
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
