use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::rest::users::mail_folders::MsgraphMailFolder;
use pimalaya_cli::printer::{Message, Printer};

use crate::msgraph::client::MsgraphClient;

/// Rename a Microsoft Graph mail folder (`PATCH /me/mailFolders/{id}`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderRenameCommand {
    /// The id of the mail folder to rename.
    #[arg(value_name = "ID")]
    pub id: String,

    /// The new display name of the mail folder.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl MsgraphMailFolderRenameCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let folder = MsgraphMailFolder {
            display_name: self.name.clone(),
            ..Default::default()
        };
        client.mail_folder_update(&self.id, &folder)?;

        printer.out(Message::new(format!(
            "Microsoft Graph mail folder `{}` successfully renamed",
            self.id
        )))
    }
}
