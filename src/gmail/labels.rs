use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::labels::{GmailLabel, GmailLabelType};
use pimalaya_cli::printer::{Message, Printer};
use serde::Serialize;

use crate::{account::context::Account, gmail::client::GmailClient};

/// Manage Gmail labels (users.labels).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailLabelsCommand {
    List(GmailLabelsListCommand),
    Get(GmailLabelGetCommand),
    Create(GmailLabelCreateCommand),
    Update(GmailLabelUpdateCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailLabelDeleteCommand),
}

impl GmailLabelsCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}

/// List all Gmail labels (users.labels.list).
#[derive(Debug, Parser)]
pub struct GmailLabelsListCommand;

impl GmailLabelsListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let labels = client.labels_list()?.response.labels;

        let table = LabelsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            colors: LabelColors {
                id: account.mailboxes_list_table_id_color(),
                name: account.mailboxes_list_table_name_color(),
                total: account.mailboxes_list_table_total_color(),
                unread: account.mailboxes_list_table_unread_color(),
            },
            labels,
        };

        printer.out(table)
    }
}

/// Get one or more Gmail labels by identifier (users.labels.get).
#[derive(Debug, Parser)]
pub struct GmailLabelGetCommand {
    /// Identifiers of the labels to get.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl GmailLabelGetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let mut labels = Vec::with_capacity(self.ids.len());

        for id in self.ids {
            labels.push(client.label_get(&id)?.response);
        }

        let table = LabelsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            colors: LabelColors {
                id: account.mailboxes_list_table_id_color(),
                name: account.mailboxes_list_table_name_color(),
                total: account.mailboxes_list_table_total_color(),
                unread: account.mailboxes_list_table_unread_color(),
            },
            labels,
        };

        printer.out(table)
    }
}

/// Create a Gmail label (users.labels.create).
#[derive(Debug, Parser)]
pub struct GmailLabelCreateCommand {
    /// Display name of the label to create.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl GmailLabelCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let label = GmailLabel {
            name: self.name.clone(),
            ..Default::default()
        };
        let label = client.label_create(&label)?.response;

        printer.out(Message::new(format!(
            "Gmail label `{}` successfully created",
            label.id
        )))
    }
}

/// Update a Gmail label name (users.labels.update).
#[derive(Debug, Parser)]
pub struct GmailLabelUpdateCommand {
    /// Identifier of the label to update.
    #[arg(value_name = "ID")]
    pub id: String,

    /// New display name to set on the label.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl GmailLabelUpdateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let label = GmailLabel {
            id: self.id.clone(),
            name: self.name.clone(),
            ..Default::default()
        };
        client.label_update(&label)?;

        printer.out(Message::new(format!(
            "Gmail label `{}` successfully updated",
            self.id
        )))
    }
}

/// Delete a Gmail label (users.labels.delete).
#[derive(Debug, Parser)]
pub struct GmailLabelDeleteCommand {
    /// Identifier of the label to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailLabelDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        client.label_delete(&self.id)?;

        printer.out(Message::new(format!(
            "Gmail label `{}` successfully deleted",
            self.id
        )))
    }
}

/// Map a label type to its Gmail wire spelling for display.
fn label_type_wire(label_type: GmailLabelType) -> &'static str {
    match label_type {
        GmailLabelType::System => "system",
        GmailLabelType::User => "user",
    }
}

/// Per-column colors for the Gmail labels table.
#[derive(Clone, Copy, Debug)]
pub struct LabelColors {
    pub id: Color,
    pub name: Color,
    pub total: Color,
    pub unread: Color,
}

impl Default for LabelColors {
    fn default() -> Self {
        Self {
            id: Color::Reset,
            name: Color::Reset,
            total: Color::Reset,
            unread: Color::Reset,
        }
    }
}

/// Renderable table of Gmail labels.
#[derive(Clone, Debug, Serialize)]
pub struct LabelsTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    #[serde(skip)]
    colors: LabelColors,
    labels: Vec<GmailLabel>,
}

impl fmt::Display for LabelsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("NAME"),
                Cell::new("TYPE"),
                Cell::new("TOTAL"),
                Cell::new("UNREAD"),
            ]))
            .add_rows(self.labels.iter().map(|label| {
                let total = label
                    .messages_total
                    .map(|n| n.to_string())
                    .unwrap_or_default();
                let unread = label
                    .messages_unread
                    .map(|n| n.to_string())
                    .unwrap_or_default();

                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&label.id).fg(self.colors.id))
                    .add_cell(Cell::new(&label.name).fg(self.colors.name))
                    .add_cell(Cell::new(
                        label.label_type.map(label_type_wire).unwrap_or_default(),
                    ))
                    .add_cell(Cell::new(total).fg(self.colors.total))
                    .add_cell(Cell::new(unread).fg(self.colors.unread));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
