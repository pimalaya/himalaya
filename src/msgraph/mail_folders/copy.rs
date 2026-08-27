use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::msgraph::client::MsgraphClient;

/// Copy a Microsoft Graph mail folder into another folder (`POST
/// /me/mailFolders/{id}/copy`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderCopyCommand {
    /// The id of the folder to copy.
    #[arg(value_name = "ID")]
    pub id: String,

    /// The destination folder id or well-known name.
    #[arg(value_name = "DESTINATION")]
    pub destination: String,
}

impl MsgraphMailFolderCopyCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let folder = client
            .mail_folder_copy(&self.id, &self.destination)?
            .response;

        printer.out(Message::new(format!(
            "Microsoft Graph mail folder copied to `{}`",
            folder.id
        )))
    }
}
