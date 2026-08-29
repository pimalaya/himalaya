//! # Gmail label list
//!
//! The `gmail labels list` command, `users.labels.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::labels::{GmailLabel, GmailLabelType};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, gmail::client::GmailClient, shared::table::style_from_preset,
};

/// List all Gmail labels (users.labels.list).
#[derive(Debug, Parser)]
pub struct GmailLabelsListCommand;

impl GmailLabelsListCommand {
    /// Lists every label and tables it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let labels = client.labels_list()?.response.labels;

        printer.out(LabelsTable::new(account, labels))
    }
}

/// Per-column colors for the Gmail labels table.
#[derive(Clone, Copy, Debug)]
pub struct LabelColors {
    /// Color of the ID column.
    pub id: Color,
    /// Color of the NAME column.
    pub name: Color,
    /// Color of the TOTAL column.
    pub total: Color,
    /// Color of the UNREAD column.
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
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct LabelsTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    #[serde(skip)]
    colors: LabelColors,
    labels: Vec<GmailLabel>,
}

impl LabelsTable {
    /// Builds the table with the account's presentation settings, shared
    /// by the `list` and `get` commands.
    pub fn new(account: &Account, labels: Vec<GmailLabel>) -> Self {
        Self {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            colors: LabelColors {
                id: account.mailboxes_list_table_id_color(),
                name: account.mailboxes_list_table_name_color(),
                total: account.mailboxes_list_table_total_color(),
                unread: account.mailboxes_list_table_unread_color(),
            },
            labels,
        }
    }
}

impl fmt::Display for LabelsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
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

/// Map a label type to its Gmail wire spelling for display.
fn label_type_wire(label_type: GmailLabelType) -> &'static str {
    match label_type {
        GmailLabelType::System => "system",
        GmailLabelType::User => "user",
    }
}
