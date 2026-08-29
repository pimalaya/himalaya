//! # Gmail attachments
//!
//! The `gmail attachments` command family, covering
//! `users.messages.attachments`.

pub mod get;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{attachments::get::GmailAttachmentGetCommand, client::GmailClient},
};

/// Manage Gmail message attachments (messages.attachments).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailAttachmentsCommand {
    Get(GmailAttachmentGetCommand),
}

impl GmailAttachmentsCommand {
    /// Runs the subcommand against the account's Gmail client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
        }
    }
}
