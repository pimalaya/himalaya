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
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use mail_parser::MessageParser;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::m2dir::{arg::M2dirNameFlag, client::M2dirClient};

/// List M2DIR envelopes from the given mailbox.
///
/// Streams every entry under the m2dir, parses each header block
/// and renders the result sorted by date.
#[derive(Debug, Parser)]
pub struct M2dirEnvelopeListCommand {
    #[command(flatten)]
    pub m2dir: M2dirNameFlag,
}

impl M2dirEnvelopeListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut M2dirClient,
    ) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;

        let entries = client.list_entries(m2dir.clone())?;
        let messages = client.read_entries_par(&m2dir, &entries)?;

        let parser = MessageParser::new();
        let mut envelopes = Vec::with_capacity(messages.len());

        for full in messages {
            let Some(parsed) = parser.parse_headers(full.contents()) else {
                continue;
            };

            envelopes.push(EnvelopesTableEntry {
                id: full.entry().id().to_owned(),
                subject: parsed.subject().unwrap_or("").to_owned(),
                from: parsed
                    .from()
                    .and_then(|a| a.first())
                    .and_then(|addr| addr.name().or(addr.address()))
                    .map(str::to_owned)
                    .unwrap_or_default(),
                date: parsed
                    .date()
                    .map(|date| date.to_rfc822())
                    .unwrap_or_default(),
            });
        }

        envelopes.sort_by(|a, b| a.date.cmp(&b.date));

        let table = EnvelopesTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            colors: EnvelopeColors {
                id: account.envelopes_list_table_id_color(),
                subject: account.envelopes_list_table_subject_color(),
                from: account.envelopes_list_table_from_color(),
                date: account.envelopes_list_table_date_color(),
            },
            envelopes,
        };

        printer.out(table)
    }
}

#[derive(Clone, Copy, Debug)]
struct EnvelopeColors {
    id: Color,
    subject: Color,
    from: Color,
    date: Color,
}

#[derive(Clone, Debug, Serialize)]
pub struct EnvelopesTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    #[serde(skip)]
    colors: EnvelopeColors,
    envelopes: Vec<EnvelopesTableEntry>,
}

impl fmt::Display for EnvelopesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("SUBJECT"),
                Cell::new("FROM"),
                Cell::new("DATE"),
            ]));

        for entry in &self.envelopes {
            let mut row = Row::new();

            row.max_height(1)
                .add_cell(Cell::new(&entry.id).fg(self.colors.id))
                .add_cell(Cell::new(&entry.subject).fg(self.colors.subject))
                .add_cell(Cell::new(&entry.from).fg(self.colors.from))
                .add_cell(Cell::new(&entry.date).fg(self.colors.date));

            table.add_row(row);
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct EnvelopesTableEntry {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub date: String,
}
