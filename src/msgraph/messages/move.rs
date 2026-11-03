//! # Microsoft Graph message move
//!
//! The `msgraph messages move` command, moving a message into another
//! folder.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{msgraph::client::MsgraphClient, shared::receipt::IdReceipt};

/// Move a Microsoft Graph message into another folder (`POST
/// /me/messages/{id}/move`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageMoveCommand {
    /// The id of the message to move.
    #[arg(value_name = "ID")]
    pub id: String,
    /// The destination folder id or well-known name.
    #[arg(value_name = "DESTINATION")]
    pub destination: String,
}

impl MsgraphMessageMoveCommand {
    /// Moves the message into the destination folder.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let message = client.message_move(&self.id, &self.destination)?.response;
        printer.out(IdReceipt {
            id: message.id,
            done: "Microsoft Graph message moved to",
        })
    }
}
