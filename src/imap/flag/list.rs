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

use std::{collections::BTreeMap, fmt};

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_imap::types::flag::{Flag, FlagPerm};
use pimalaya_cli::printer::Printer;
use serde::{Serialize, Serializer};

use crate::account::context::Account;
use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

/// List available IMAP flags for the given mailbox.
///
/// This command displays the flags and permanent flags that are
/// available in the given mailbox. These flags come from the SELECT
/// response.
#[derive(Debug, Parser)]
pub struct ImapFlagListCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapFlagListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        let data = client.select(mailbox)?;
        let flags = data.flags.unwrap_or_default();
        let permanent_flags = data.permanent_flags.unwrap_or_default();

        let table = FlagsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            flags,
            permanent_flags,
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FlagsTable<'a> {
    #[serde(skip_serializing)]
    preset: String,
    #[serde(skip_serializing)]
    arrangement: ContentArrangement,
    #[serde(serialize_with = "serialize_flags")]
    flags: Vec<Flag<'a>>,
    #[serde(serialize_with = "serialize_permanent_flags")]
    permanent_flags: Vec<FlagPerm<'a>>,
}

impl FlagsTable<'_> {
    fn build_entries(&self) -> Vec<(String, bool)> {
        let mut entries: BTreeMap<String, bool> = BTreeMap::new();

        for flag in &self.flags {
            entries.entry(flag.to_string()).or_insert(false);
        }

        for flag in &self.permanent_flags {
            let name = match flag {
                FlagPerm::Flag(f) => f.to_string(),
                FlagPerm::Asterisk => "\\*".to_string(),
            };
            entries.insert(name, true);
        }

        entries.into_iter().collect()
    }
}

impl fmt::Display for FlagsTable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("FLAG"), Cell::new("PERMANENT")]));

        for (flag, perm) in self.build_entries() {
            table.add_row(Row::from([
                Cell::new(&flag),
                Cell::new(if perm { "true" } else { "" }),
            ]));
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

pub fn serialize_flags<S: Serializer>(
    flags: &Vec<Flag<'_>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    flags
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .serialize(serializer)
}

fn serialize_permanent_flags<S: Serializer>(
    flags: &Vec<FlagPerm<'_>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    flags
        .iter()
        .map(|f| match f {
            FlagPerm::Flag(f) => f.to_string(),
            FlagPerm::Asterisk => "\\*".to_string(),
        })
        .collect::<Vec<_>>()
        .serialize(serializer)
}
