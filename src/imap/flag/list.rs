use std::{collections::BTreeMap, fmt};

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_imap::{
    rfc3501::select::*,
    types::flag::{Flag, FlagPerm},
};
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::{Serialize, Serializer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

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
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapMailboxSelect::new(imap.context, mailbox);

        let (flags, permanent_flags) = loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxSelectResult::Io { input } => {
                    arg = Some(handle(&mut imap.stream, input)?)
                }
                ImapMailboxSelectResult::Ok { data, .. } => {
                    break (
                        data.flags.unwrap_or_default(),
                        data.permanent_flags.unwrap_or_default(),
                    )
                }
                ImapMailboxSelectResult::Err { err, .. } => bail!(err),
            }
        };

        let table = FlagsTable {
            preset: account.table_preset,
            arrangement: account.table_arrangement,
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
