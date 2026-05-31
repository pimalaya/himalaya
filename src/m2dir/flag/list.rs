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
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::m2dir::{
    arg::{M2dirNameFlag, MessageIdArg},
    client::M2dirClient,
};

/// List flags set on an m2dir message.
///
/// Reads the `.meta/<id>.flags` metadata file and prints one flag per
/// row. Returns an empty table when the file is missing.
#[derive(Debug, Parser)]
pub struct M2dirFlagListCommand {
    #[command(flatten)]
    pub m2dir: M2dirNameFlag,
    #[command(flatten)]
    pub id: MessageIdArg,
}

impl M2dirFlagListCommand {
    pub fn execute(self, printer: &mut impl Printer, client: M2dirClient) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;
        let flags = client.read_flags(&m2dir, &self.id.inner)?;

        let table = FlagsTable {
            preset: client.account.table_preset().to_string(),
            arrangement: client.account.table_arrangement(),
            flags: flags.iter().map(str::to_owned).collect(),
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
    flags: Vec<String>,
}

impl fmt::Display for FlagsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("FLAG")]));

        for flag in &self.flags {
            table.add_row(Row::from([Cell::new(flag)]));
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
