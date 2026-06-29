use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_msgraph::v1::rest::users::mail_folders::{
    MsgraphMailFolder, list::MsgraphMailFoldersListParams,
};
use pimalaya_cli::printer::Printer;
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
        let folders = client.mail_folders_list(&params)?.response.value;

        printer.out(folders_table(account, folders))
    }
}

/// Builds the renderable folders table with the account's mailbox-table
/// colors and layout.
pub(crate) fn folders_table(
    account: &Account,
    folders: Vec<MsgraphMailFolder>,
) -> MailFoldersTable {
    MailFoldersTable {
        preset: account.table_preset().to_string(),
        arrangement: account.table_arrangement(),
        colors: MailFolderColors {
            id: account.mailboxes_list_table_id_color(),
            name: account.mailboxes_list_table_name_color(),
            total: account.mailboxes_list_table_total_color(),
            unread: account.mailboxes_list_table_unread_color(),
        },
        folders,
    }
}

/// Per-column colors for the Microsoft Graph mail folders table.
#[derive(Clone, Copy, Debug)]
pub struct MailFolderColors {
    pub id: Color,
    pub name: Color,
    pub total: Color,
    pub unread: Color,
}

impl Default for MailFolderColors {
    fn default() -> Self {
        Self {
            id: Color::Reset,
            name: Color::Reset,
            total: Color::Reset,
            unread: Color::Reset,
        }
    }
}

/// Renderable table of Microsoft Graph mail folders.
#[derive(Clone, Debug, Serialize)]
pub struct MailFoldersTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    #[serde(skip)]
    colors: MailFolderColors,
    folders: Vec<MsgraphMailFolder>,
}

impl fmt::Display for MailFoldersTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("NAME"),
                Cell::new("TOTAL"),
                Cell::new("UNREAD"),
                Cell::new("CHILDREN"),
            ]))
            .add_rows(self.folders.iter().map(|folder| {
                let total = folder
                    .total_item_count
                    .map(|n| n.to_string())
                    .unwrap_or_default();
                let unread = folder
                    .unread_item_count
                    .map(|n| n.to_string())
                    .unwrap_or_default();
                let children = folder
                    .child_folder_count
                    .map(|n| n.to_string())
                    .unwrap_or_default();

                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&folder.id).fg(self.colors.id))
                    .add_cell(Cell::new(&folder.display_name).fg(self.colors.name))
                    .add_cell(Cell::new(total).fg(self.colors.total))
                    .add_cell(Cell::new(unread).fg(self.colors.unread))
                    .add_cell(Cell::new(children));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
