use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    msgraph::{client::MsgraphClient, mail_folder::list::folders_table},
};

/// Get one or more Microsoft Graph mail folders by id (`GET
/// /me/mailFolders/{id}`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderGetCommand {
    /// The ids of the mail folders to get.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl MsgraphMailFolderGetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let mut folders = Vec::with_capacity(self.ids.len());

        for id in self.ids {
            folders.push(client.mail_folder_get(&id)?.response);
        }

        printer.out(folders_table(account, folders))
    }
}
