//! # IMAP flag list
//!
//! The `imap flag` command, reading the flags a `SELECT` response
//! reports.

use io_imap::client::ImapClient as _;
use std::{collections::BTreeMap, fmt};

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_imap::{
    rfc3501::select::ImapMailboxSelectOptions,
    types::flag::{Flag, FlagPerm},
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::{Serialize, Serializer};

use crate::{
    account::context::Account,
    imap::{client::ImapClient, mailbox::arg::MailboxNameArg},
    shared::table::style_from_preset,
};

/// List the flags a mailbox allows (SELECT response, RFC 3501).
///
/// The `FLAGS` and `PERMANENTFLAGS` the server returns on selecting it.
#[derive(Debug, Parser)]
pub struct ImapFlagListCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapFlagListCommand {
    /// Selects the mailbox and tables the flags it reported.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        let data = client.select(mailbox, ImapMailboxSelectOptions::default())?;
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

/// The `imap flag` output, a table of the flags a mailbox allows and of
/// the ones it keeps across sessions.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct FlagsTable<'a> {
    #[serde(skip_serializing)]
    #[schemars(skip)]
    preset: String,
    #[serde(skip_serializing)]
    #[schemars(skip)]
    arrangement: ContentArrangement,
    #[serde(serialize_with = "serialize_flags")]
    #[schemars(with = "Vec<String>")]
    flags: Vec<Flag<'a>>,
    #[serde(serialize_with = "serialize_permanent_flags")]
    #[schemars(with = "Vec<String>")]
    permanent_flags: Vec<FlagPerm<'a>>,
}

impl FlagsTable<'_> {
    /// Pairs each flag with whether it is permanent.
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
            .load_style(style_from_preset(&self.preset))
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

/// Serializes flags as their wire spellings.
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

/// Serializes permanent flags as their wire spellings.
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
