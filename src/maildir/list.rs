//! # Maildir list
//!
//! The `maildir list` command, tabling the Maildirs under the store
//! root.

use std::{fmt, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, Row, Table};
use io_maildir::maildir::Maildir;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, maildir::client::MaildirClient, shared::table::style_from_preset,
};

/// List Maildir folders.
///
/// Scans the account root and lists every folder found, with its name
/// and filesystem path.
#[derive(Debug, Parser)]
pub struct MaildirMailboxListCommand;

impl MaildirMailboxListCommand {
    /// Lists the Maildirs under the store root and tables them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MaildirClient,
    ) -> Result<()> {
        let maildirs = client.list_maildirs()?;

        let table = MaildirsTable {
            preset: account.table_preset().to_string(),
            name_color: account.mailboxes_list_table_name_color(),
            rows: maildirs.into_iter().map(From::from).collect(),
        };

        printer.out(table)
    }
}

/// The `maildir list` output, a table of Maildirs.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct MaildirsTable {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The color of the NAME column.
    #[serde(skip)]
    pub name_color: Color,
    /// The Maildirs found under the store root.
    #[serde(rename = "maildirs")]
    pub rows: Vec<MaildirRow>,
}

impl fmt::Display for MaildirsTable {
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

/// One row of the Maildirs table.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct MaildirRow {
    /// The Maildir name, relative to the store root.
    pub name: String,
    /// Its filesystem path.
    pub path: PathBuf,
}

impl From<Maildir> for MaildirRow {
    fn from(maildir: Maildir) -> Self {
        Self {
            name: maildir.name().unwrap_or("Unknown").to_owned(),
            path: PathBuf::from(maildir.path().as_str()),
        }
    }
}
