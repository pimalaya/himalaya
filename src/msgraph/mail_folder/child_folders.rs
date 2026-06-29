use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::rest::users::mail_folders::list::MsgraphMailFoldersListParams;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    msgraph::{client::MsgraphClient, mail_folder::list::folders_table},
};

/// List a mail folder's child folders (`GET
/// /me/mailFolders/{id}/childFolders`).
#[derive(Debug, Parser)]
pub struct MsgraphChildFoldersListCommand {
    /// The id or well-known name of the parent folder.
    #[arg(value_name = "ID")]
    pub id: String,

    /// Maximum number of folders to return (OData `$top`).
    #[arg(long, value_name = "N")]
    pub top: Option<u32>,

    /// Number of folders to skip (OData `$skip`).
    #[arg(long, value_name = "N")]
    pub skip: Option<u32>,

    /// OData `$select`: comma-separated fields to return (e.g.
    /// `displayName,totalItemCount`).
    #[arg(long, value_name = "FIELDS")]
    pub select: Option<String>,

    /// Also include hidden folders.
    #[arg(long)]
    pub hidden: bool,
}

impl MsgraphChildFoldersListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let params = MsgraphMailFoldersListParams {
            top: self.top,
            skip: self.skip,
            select: self.select.as_deref(),
            include_hidden_folders: self.hidden.then_some(true),
        };
        let folders = client.child_folders_list(&self.id, &params)?.response.value;

        printer.out(folders_table(account, folders))
    }
}
