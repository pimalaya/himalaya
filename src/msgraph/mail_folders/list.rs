//! # Microsoft Graph mail folder list
//!
//! The `msgraph mail-folders list` command, listing the folders of the
//! mailbox.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::rest::users::mail_folders::{
    MsgraphMailFolder, list::MsgraphMailFoldersListParams,
};
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, msgraph::client::MsgraphClient};

/// List Microsoft Graph mail folders (`GET /me/mailFolders`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderListCommand {
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

impl MsgraphMailFolderListCommand {
    /// Lists one page of folders and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let params = MsgraphMailFoldersListParams {
            top: self.top,
            skip: self.skip,
            select: self.select.as_deref(),
            include_hidden_folders: self.hidden.then_some(true),
        };
        let folders = client.mail_folders_list(&params)?.response.value;

        printer.out(MsgraphMailFolderListOutput::new(folders))
    }
}

/// The Microsoft Graph mail folders, as `list`, `get` and
/// `child-folders` print them.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MsgraphMailFolderListOutput {
    folders: Vec<MsgraphMailFolder>,
}

impl MsgraphMailFolderListOutput {
    /// Wraps the folders the server returned.
    pub fn new(folders: Vec<MsgraphMailFolder>) -> Self {
        Self { folders }
    }
}

impl fmt::Display for MsgraphMailFolderListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: a name is free text, so it reads last.
        let mut columns = Columns::new(["ID", "TOTAL", "UNREAD", "CHILDREN", "NAME"]);

        for folder in &self.folders {
            columns.push([
                sanitize::text(&folder.id).into_owned(),
                count(folder.total_item_count),
                count(folder.unread_item_count),
                count(folder.child_folder_count),
                sanitize::text(&folder.display_name).into_owned(),
            ]);
        }

        write!(f, "{columns}")
    }
}

/// Renders a count, or an empty cell when Graph gave none.
fn count(value: Option<impl ToString>) -> String {
    value.map(|n| n.to_string()).unwrap_or_default()
}
