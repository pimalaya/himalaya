use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::msgraph::client::MsgraphClient;

/// Delete a Microsoft Graph mail folder and everything in it (`DELETE
/// /me/mailFolders/{id}`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderDeleteCommand {
    #[arg(value_name = "ID")]
    pub id: String,
}

impl MsgraphMailFolderDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        client.mail_folder_delete(&self.id)?;

        printer.out(Message::new(format!(
            "Microsoft Graph mail folder `{}` successfully deleted",
            self.id
        )))
    }
}
