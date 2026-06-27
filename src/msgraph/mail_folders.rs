use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_msgraph::v1::rest::users::mail_folders::{
    MsgraphMailFolder, list::MsgraphMailFoldersListParams,
};
use pimalaya_cli::printer::{Message, Printer};
use serde::Serialize;

use crate::{account::context::Account, msgraph::client::MsgraphClient};

/// Manage Microsoft Graph mail folders (`me.mailFolders`).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MsgraphMailFoldersCommand {
    List(MsgraphMailFoldersListCommand),
    Get(MsgraphMailFolderGetCommand),
    Create(MsgraphMailFolderCreateCommand),
    Rename(MsgraphMailFolderRenameCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(MsgraphMailFolderDeleteCommand),
}

impl MsgraphMailFoldersCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Rename(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}

/// List Microsoft Graph mail folders (`GET /me/mailFolders`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFoldersListCommand {
    /// Maximum number of folders to return (OData `$top`).
    #[arg(short = 's', long, value_name = "N")]
    pub top: Option<u32>,

    /// Number of folders to skip (OData `$skip`).
    #[arg(long, value_name = "N")]
    pub skip: Option<u32>,

    /// Also include hidden folders.
    #[arg(long)]
    pub hidden: bool,
}

impl MsgraphMailFoldersListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let params = MsgraphMailFoldersListParams {
            top: self.top,
            skip: self.skip,
            select: None,
            include_hidden_folders: self.hidden.then_some(true),
        };
        let folders = client.mail_folders_list(&params)?.response.value;

        printer.out(folders_table(account, folders))
    }
}

/// Get one or more Microsoft Graph mail folders by id (`GET
/// /me/mailFolders/{id}`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderGetCommand {
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

/// Create a Microsoft Graph mail folder under the mailbox root (`POST
/// /me/mailFolders`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderCreateCommand {
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

/// Rename a Microsoft Graph mail folder (`PATCH /me/mailFolders/{id}`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderRenameCommand {
    #[arg(value_name = "ID")]
    pub id: String,

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

/// Builds the renderable folders table with the account's mailbox-table
/// colors and layout.
fn folders_table(account: &Account, folders: Vec<MsgraphMailFolder>) -> MailFoldersTable {
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
