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

use std::{fmt, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, Row, Table};
use io_maildir::maildir::Maildir;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::maildir::client::MaildirClient;

/// List, search and filter maildirs.
///
/// This command allows you to list maildirs from your MAILDIR account.
/// By default, only subscribed maildirs are listed. Use --all to
/// list all maildirs.
#[derive(Debug, Parser)]
pub struct MaildirMailboxListCommand;

impl MaildirMailboxListCommand {
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

#[derive(Clone, Debug, Serialize)]
pub struct MaildirsTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub name_color: Color,
    #[serde(rename = "maildirs")]
    pub rows: Vec<MaildirRow>,
}

impl fmt::Display for MaildirsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
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

#[derive(Clone, Debug, Serialize)]
pub struct MaildirRow {
    pub name: String,
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
