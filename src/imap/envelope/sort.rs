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

use anyhow::{Result, bail};
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table, presets};
use io_imap::types::{
    core::Vec1,
    extensions::sort::{SortCriterion, SortKey},
};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::imap::{
    client::ImapClient, envelope::search::parse_query, mailbox::arg::MailboxNameOptionalArg,
};

/// Sort messages by criteria.
///
/// This command searches for messages matching the given query and
/// returns them sorted by the specified criteria. Requires the SORT
/// IMAP extension.
///
/// Sort criteria:
///   - date      - sort by Date header
///   - arrival   - sort by internal date (arrival time)
///   - from      - sort by From header
///   - to        - sort by To header
///   - cc        - sort by Cc header
///   - subject   - sort by Subject header
///   - size      - sort by message size
#[derive(Debug, Parser)]
pub struct ImapEnvelopeSortCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalArg,

    /// Sort criteria (e.g., "date", "from", "subject", "size").
    #[arg(short = 'S', long, default_value = "date")]
    pub sort: String,

    /// Reverse sort order.
    #[arg(short, long)]
    pub reverse: bool,

    /// Search query (same syntax as search command).
    #[arg(name = "query", value_name = "QUERY", default_value = "all")]
    pub query: String,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapEnvelopeSortCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        client.select(mailbox)?;

        let sort_key = parse_sort_key(&self.sort)?;
        let sort_criteria = Vec1::unvalidated(vec![SortCriterion {
            reverse: self.reverse,
            key: sort_key,
        }]);
        let search_criteria = parse_query(&self.query)?;

        let ids = client.sort(sort_criteria, search_criteria, !self.seq)?;

        let id_color = account.envelopes_list_table_id_color();
        let table = SortResultsTable::new(ids, !self.seq, id_color);

        printer.out(table)?;
        Ok(())
    }
}

fn parse_sort_key(s: &str) -> Result<SortKey> {
    match s.to_lowercase().as_str() {
        "date" => Ok(SortKey::Date),
        "arrival" => Ok(SortKey::Arrival),
        "from" => Ok(SortKey::From),
        "to" => Ok(SortKey::To),
        "cc" => Ok(SortKey::Cc),
        "subject" => Ok(SortKey::Subject),
        "size" => Ok(SortKey::Size),
        _ => bail!(
            "Unknown sort key `{s}`, valid options: date, arrival, from, to, cc, subject, size"
        ),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SortResultsTable {
    ids: Vec<u32>,
    uid_mode: bool,
    #[serde(skip)]
    id_color: Color,
}

impl SortResultsTable {
    pub fn new(ids: Vec<std::num::NonZeroU32>, uid_mode: bool, id_color: Color) -> Self {
        let ids = ids.into_iter().map(|id| id.get()).collect();
        Self {
            ids,
            uid_mode,
            id_color,
        }
    }
}

impl fmt::Display for SortResultsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        let id_header = if self.uid_mode { "UID" } else { "SEQ" };

        table
            .load_preset(presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([Cell::new(id_header)]));

        for id in &self.ids {
            table.add_row(Row::from([Cell::new(id).fg(self.id_color)]));
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        writeln!(f, "Found {} message(s)", self.ids.len())
    }
}
