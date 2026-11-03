//! # Gmail message delete
//!
//! The `gmail messages delete` command, `users.messages.delete`.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{gmail::client::GmailClient, shared::receipt::IdReceipt};

/// Permanently delete a Gmail message (users.messages.delete).
#[derive(Debug, Parser)]
pub struct GmailMessageDeleteCommand {
    /// The id of the message to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailMessageDeleteCommand {
    /// Deletes the message for good.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        client.message_delete(&self.id)?;
        printer.out(IdReceipt {
            id: self.id,
            done: "Gmail message permanently deleted",
        })
    }
}
