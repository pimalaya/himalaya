//! # Microsoft Graph mail folder move
//!
//! The `msgraph mail-folders move` command, moving a folder into another.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{msgraph::client::MsgraphClient, shared::receipt::IdReceipt};

/// Move a Microsoft Graph mail folder into another folder (`POST
/// /me/mailFolders/{id}/move`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderMoveCommand {
    /// The id of the folder to move.
    #[arg(value_name = "ID")]
    pub id: String,
    /// The destination folder id or well-known name.
    #[arg(value_name = "DESTINATION")]
    pub destination: String,
}

impl MsgraphMailFolderMoveCommand {
    /// Moves the folder into the destination one.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let folder = client
            .mail_folder_move(&self.id, &self.destination)?
            .response;

        printer.out(IdReceipt {
            id: folder.id,
            done: "Microsoft Graph mail folder moved to",
        })
    }
}
