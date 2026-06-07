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
use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_maildir::flag::types::MaildirFlag;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;

/// List available MAILDIR flags for the given mailbox.
///
/// This command displays the flags and permanent flags that are
/// available in the given mailbox. These flags come from the SELECT
/// response.
#[derive(Debug, Parser)]
pub struct MaildirFlagListCommand;

impl MaildirFlagListCommand {
    pub fn execute(self, printer: &mut impl Printer, account: &mut Account) -> Result<()> {
        let table = FlagsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            flags: vec![
                FlagRow::new(MaildirFlag::Passed),
                FlagRow::new(MaildirFlag::Replied),
                FlagRow::new(MaildirFlag::Seen),
                FlagRow::new(MaildirFlag::Trashed),
                FlagRow::new(MaildirFlag::Draft),
                FlagRow::new(MaildirFlag::Flagged),
            ],
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FlagsTable {
    #[serde(skip_serializing)]
    preset: String,
    #[serde(skip_serializing)]
    arrangement: ContentArrangement,
    flags: Vec<FlagRow>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FlagRow {
    code: String,
    name: String,
}

impl FlagRow {
    pub fn new(flag: MaildirFlag) -> Self {
        Self {
            code: flag.to_string(),
            name: format!("{flag:?}"),
        }
    }
}

impl fmt::Display for FlagsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("CODE"), Cell::new("NAME")]));

        for flag in &self.flags {
            table.add_row(Row::from([Cell::new(&flag.code), Cell::new(&flag.name)]));
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
