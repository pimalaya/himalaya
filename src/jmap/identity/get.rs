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
use comfy_table::{Cell, Row, Table};
use io_jmap::rfc8621::identity::Identity;
use log::warn;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::jmap::client::JmapClient;

/// Get JMAP identities (Identity/get).
///
/// Lists sender identities available for sending email. Pass no IDs to
/// list all identities.
#[derive(Debug, Parser)]
pub struct JmapIdentityGetCommand {
    /// Identity ID(s) to retrieve (omit to get all).
    #[arg(value_name = "ID")]
    pub ids: Option<Vec<String>>,
}

impl JmapIdentityGetCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: JmapClient) -> Result<()> {
        let output = client.identity_get(self.ids)?;

        for id in output.not_found {
            warn!("identity `{id}` not found");
        }

        let table = IdentitiesTable {
            preset: client.account.table_preset().to_string(),
            identities: output.identities,
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct IdentitiesTable {
    #[serde(skip)]
    pub preset: String,
    pub identities: Vec<Identity>,
}

impl fmt::Display for IdentitiesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("NAME"),
                Cell::new("EMAIL"),
            ]))
            .add_rows(
                self.identities.iter().map(|i| {
                    Row::from([Cell::new(&i.id), Cell::new(&i.name), Cell::new(&i.email)])
                }),
            );

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
