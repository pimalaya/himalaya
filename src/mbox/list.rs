//! # mbox list
//!
//! The `mbox list` command, tabling the mbox files of the account.

use std::{fmt, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use io_mbox::mbox::Mbox;
use pimalaya_cli::{
    printer::Printer,
    table::{Cell, Color, Row, Table},
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, mbox::client::MboxClient, shared::table::style_from_preset,
};

/// List mbox files.
///
/// Walks the account root and lists every mbox found, with its name and
/// filesystem path, plus the spool as `INBOX` when one is configured.
#[derive(Debug, Parser)]
pub struct MboxMailboxListCommand;

impl MboxMailboxListCommand {
    /// Lists the mbox files and tables them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MboxClient,
    ) -> Result<()> {
        let mboxes = client.list_mboxes()?;

        let table = MboxesTable {
            preset: account.table_preset().to_string(),
            name_color: account.mailboxes_list_table_name_color(),
            rows: mboxes.into_iter().map(From::from).collect(),
        };

        printer.out(table)
    }
}

/// The `mbox list` output, a table of mbox files.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct MboxesTable {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The color of the NAME column.
    #[serde(skip)]
    pub name_color: Color,
    /// The mbox files found.
    #[serde(rename = "mboxes")]
    pub rows: Vec<MboxRow>,
}

impl fmt::Display for MboxesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
            .set_header(Row::from([Cell::new("NAME"), Cell::new("PATH")]))
            .add_rows(self.rows.iter().map(|m| {
                let mut row = Row::new();

                row.max_height(1)
                    .add_cell(Cell::new(&m.name).fg(self.name_color))
                    .add_cell(Cell::new(format!("{}", m.path.display())));

                row
            }));

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

/// One row of the mbox table.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct MboxRow {
    /// The mbox name, relative to the account root.
    pub name: String,
    /// Its filesystem path.
    pub path: PathBuf,
}

impl From<Mbox> for MboxRow {
    fn from(mbox: Mbox) -> Self {
        Self {
            name: mbox.name.to_string(),
            path: PathBuf::from(mbox.path.as_str()),
        }
    }
}
