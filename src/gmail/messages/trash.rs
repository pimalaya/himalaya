//! # Gmail message trash
//!
//! The `gmail messages trash` command, `users.messages.trash`.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{gmail::client::GmailClient, shared::receipt::IdReceipt};

/// Move a Gmail message to the trash (users.messages.trash).
#[derive(Debug, Parser)]
pub struct GmailMessageTrashCommand {
    /// The id of the message to trash.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailMessageTrashCommand {
    /// Moves the message to the trash.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let message = client.message_trash(&self.id)?.response;
        printer.out(IdReceipt {
            id: message.id,
            done: "Gmail message successfully trashed",
        })
    }
}
