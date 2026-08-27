use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::rest::users::mail_folders::MsgraphMailFolder;
use pimalaya_cli::printer::{Message, Printer};

use crate::msgraph::client::MsgraphClient;

/// Create a Microsoft Graph mail folder under the mailbox root (`POST
/// /me/mailFolders`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderCreateCommand {
    /// The display name of the mail folder to create.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl MsgraphMailFolderCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let folder = MsgraphMailFolder {
            display_name: self.name.clone(),
            ..Default::default()
        };
        let folder = client.mail_folder_create(&folder)?.response;

        printer.out(Message::new(format!(
            "Microsoft Graph mail folder `{}` successfully created",
            folder.id
        )))
    }
}
