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
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::maildir::{arg::MaildirPathFlag, client::MaildirClient};

/// List MAILDIR envelopes from the given mailbox.
///
/// This command displays envelopes for messages in the specified
/// mailbox. You can specify a sequence set to limit which messages
/// are fetched.
#[derive(Debug, Parser)]
pub struct MaildirEnvelopeListCommand {
    #[command(flatten)]
    pub maildir: MaildirPathFlag,
}

impl MaildirEnvelopeListCommand {
    pub fn execute(self, printer: &mut impl Printer, client: MaildirClient) -> Result<()> {
        let maildir = client.resolve_maildir(&self.maildir.inner)?;

        let messages = client.list_messages(maildir)?;

        let mut envelopes = Vec::with_capacity(messages.len());

        for message in messages {
            let Some(id) = message.id() else {
                continue;
            };
            let id = id.to_owned();

            let Some(parsed) = message.headers() else {
                continue;
            };

            let mut row = EnvelopesTableEntry::default();

            row.id = id;
            row.subject = parsed.subject().unwrap_or("").to_owned();

            if let Some(addr) = parsed.from().and_then(|a| a.first()) {
                row.from = addr.name().or(addr.address()).unwrap_or("").to_owned();
            }

            if let Some(date) = parsed.date() {
                row.date = date.to_rfc822();
            }

            envelopes.push(row);
        }

        envelopes.sort_by(|a, b| a.date.cmp(&b.date));

        let table = EnvelopesTable {
            preset: client.account.table_preset().to_string(),
            arrangement: client.account.table_arrangement(),
            colors: EnvelopeColors {
                id: client.account.envelopes_list_table_id_color(),
                subject: client.account.envelopes_list_table_subject_color(),
                from: client.account.envelopes_list_table_from_color(),
                date: client.account.envelopes_list_table_date_color(),
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
