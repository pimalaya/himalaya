//! # Microsoft Graph message delete
//!
//! The `msgraph messages delete` command, deleting a message for good.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{msgraph::client::MsgraphClient, shared::receipt::IdReceipt};

/// Permanently delete a Microsoft Graph message (`DELETE
/// /me/messages/{id}`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageDeleteCommand {
    /// The id of the message to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl MsgraphMessageDeleteCommand {
    /// Deletes the message for good.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        client.message_delete(&self.id)?;
        printer.out(IdReceipt {
            id: self.id,
            done: "Microsoft Graph message permanently deleted",
        })
    }
}
