// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, Row, Table};
use io_m2dir::m2dir::M2dir;
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
        let m2dirs = client.list_mailboxes()?;

        let table = M2dirsTable {
            preset: account.table_preset().to_string(),
            name_color: account.mailboxes_list_table_name_color(),
            rows: m2dirs.into_iter().map(From::from).collect(),
        };

        printer.out(table)
    }
}

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

#[derive(Clone, Debug, Serialize)]
pub struct M2dirRow {
    pub name: String,
    pub path: String,
}

impl From<M2dir> for M2dirRow {
    fn from(m2dir: M2dir) -> Self {
        let name = m2dir
            .path()
            .file_name()
            .map(str::to_owned)
            .unwrap_or_default();
        let path = m2dir.path().as_str().to_owned();
        Self { name, path }
    }
}
