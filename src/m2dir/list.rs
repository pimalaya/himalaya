use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, Row, Table};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::m2dir::client::M2dirClient;

/// List m2dir folders found under the store root.
#[derive(Debug, Parser)]
pub struct M2dirMailboxListCommand;

impl M2dirMailboxListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut M2dirClient,
    ) -> Result<()> {
        let store = client.open_store()?;
        let m2dirs = client.list_m2dirs()?;

        let rows = m2dirs
            .into_iter()
            .map(|m2dir| {
                let name = store
                    .decode_folder_name(m2dir.path())
                    .unwrap_or_else(|| m2dir.path().as_str().to_owned());
                let path = m2dir.path().as_str().to_owned();
                M2dirRow { name, path }
            })
            .collect();

        let table = M2dirsTable {
            preset: account.table_preset().to_string(),
            name_color: account.mailboxes_list_table_name_color(),
            rows,
        };

        printer.out(table)
    }
}

/// Renderable table of m2dir folders.
#[derive(Clone, Debug, Serialize)]
pub struct M2dirsTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub name_color: Color,
    #[serde(rename = "m2dirs")]
    pub rows: Vec<M2dirRow>,
}

impl fmt::Display for M2dirsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([Cell::new("NAME"), Cell::new("PATH")]))
            .add_rows(self.rows.iter().map(|m| {
                let mut row = Row::new();

                row.max_height(1)
                    .add_cell(Cell::new(&m.name).fg(self.name_color))
                    .add_cell(Cell::new(&m.path));

                row
            }));

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

/// One row of the m2dir folders table: name and path.
#[derive(Clone, Debug, Serialize)]
pub struct M2dirRow {
    pub name: String,
    pub path: String,
}
