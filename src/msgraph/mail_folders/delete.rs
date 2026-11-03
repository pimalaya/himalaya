//! # Microsoft Graph mail folder delete
//!
//! The `msgraph mail-folders delete` command, deleting a folder and
//! everything in it.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{msgraph::client::MsgraphClient, shared::receipt::IdReceipt};

/// Delete a Microsoft Graph mail folder and everything in it (`DELETE
/// /me/mailFolders/{id}`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderDeleteCommand {
    /// The id of the mail folder to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl MsgraphMailFolderDeleteCommand {
    /// Deletes the folder and everything in it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        client.mail_folder_delete(&self.id)?;

        printer.out(IdReceipt {
            id: self.id,
            done: "Microsoft Graph mail folder successfully deleted",
        })
    }
}
