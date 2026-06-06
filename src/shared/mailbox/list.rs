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
use io_email::mailbox::types::Mailbox;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::shared::client::EmailClient;

/// Shared API to list mailboxes for the active account.
#[derive(Debug, Parser)]
pub struct MailboxListCommand {
    /// Populate per-mailbox message counts (TOTAL and UNREAD columns).
    ///
    /// JMAP returns counts in the same response. IMAP issues an
    /// extra `STATUS` per mailbox, which can be slow on accounts
    /// with many mailboxes. Maildir does not implement counts yet.
    #[arg(long)]
    pub counts: bool,

    /// Maximum width of the rendered table, in terminal columns.
    ///
    /// Overrides comfy-table's auto-detection. Columns shrink with
    /// ellipsis if needed. Useful when piping through `less -S` or
    /// rendering into a fixed-width log.
    #[arg(long = "max-width", short = 'w')]
    #[arg(value_name = "COLUMNS")]
    pub max_width: Option<u16>,
}

impl MailboxListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailboxes = client.list_mailboxes(self.counts)?;

        let mailboxes = Mailboxes {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            max_width: self.max_width,
            with_counts: self.counts,
            colors: MailboxColors {
                id: account.mailboxes_list_table_id_color(),
                name: account.mailboxes_list_table_name_color(),
                total: account.mailboxes_list_table_total_color(),
                unread: account.mailboxes_list_table_unread_color(),
            },
            mailboxes,
        };

        printer.out(mailboxes)
    }
}

#[derive(Clone, Copy, Debug)]
struct MailboxColors {
    id: Color,
    name: Color,
    total: Color,
    unread: Color,
}

#[derive(Clone, Debug, Serialize)]
pub struct Mailboxes {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    #[serde(skip)]
    pub max_width: Option<u16>,
    #[serde(skip)]
    pub with_counts: bool,
    #[serde(skip)]
    colors: MailboxColors,
    pub mailboxes: Vec<Mailbox>,
}

impl fmt::Display for Mailboxes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        let mut header = vec![Cell::new("ID"), Cell::new("NAME")];
        if self.with_counts {
            header.push(Cell::new("TOTAL"));
            header.push(Cell::new("UNREAD"));
        }

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from(header))
            .add_rows(self.mailboxes.iter().map(|m| {
                let mut row = Row::new();
                row.max_height(1);
                row.add_cell(Cell::new(&m.id).fg(self.colors.id));
                row.add_cell(Cell::new(&m.name).fg(self.colors.name));
                if self.with_counts {
                    row.add_cell(count_cell(m.total).fg(self.colors.total));
                    row.add_cell(count_cell(m.unread).fg(self.colors.unread));
                }
                row
            }));

        if let Some(width) = self.max_width {
            table.set_width(width);
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

fn count_cell(value: Option<u64>) -> Cell {
    match value {
        Some(n) => Cell::new(n),
        None => Cell::new(""),
    }
}
