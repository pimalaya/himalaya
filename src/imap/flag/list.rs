use std::{collections::BTreeMap, fmt};

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{presets, Cell, ContentArrangement, Row, Table};
use io_imap::{
    coroutines::select::*,
    types::flag::{Flag, FlagPerm},
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::{Serialize, Serializer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameOptionalArg, stream};

/// List available flags for a mailbox.
///
/// This command displays the flags and permanent flags that are
/// available in the given mailbox. These flags come from the SELECT
/// response.
#[derive(Debug, Parser)]
pub struct ListFlagsCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalArg,
}

impl ListFlagsCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let mailbox = self.mailbox.name.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapSelect::new(context, mailbox);

        let (flags, permanent_flags) = loop {
            match coroutine.resume(arg.take()) {
                ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapSelectResult::Ok { data, .. } => {
                    break (
                        data.flags.unwrap_or_default(),
                        data.permanent_flags.unwrap_or_default(),
                    )
                }
                ImapSelectResult::Err { err, .. } => bail!(err),
            }
        };

        let table = FlagsTable {
            flags,
            permanent_flags,
        };

        printer.out(table)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FlagEntry {
    pub name: String,
    pub permanent: bool,
}

pub struct FlagsTable {
    flags: Vec<Flag<'static>>,
    permanent_flags: Vec<FlagPerm<'static>>,
}

impl FlagsTable {
    fn build_entries(&self) -> Vec<FlagEntry> {
        let mut entries: BTreeMap<String, bool> = BTreeMap::new();

        // Add flags
        for flag in &self.flags {
            entries.entry(flag.to_string()).or_insert(false);
        }

        // Mark permanent flags
        for flag in &self.permanent_flags {
            let name = match flag {
                FlagPerm::Flag(f) => f.to_string(),
                FlagPerm::Asterisk => "\\*".to_string(),
            };
            entries.insert(name, true);
        }

        entries
            .into_iter()
            .map(|(name, permanent)| FlagEntry { name, permanent })
            .collect()
    }
}

impl fmt::Display for FlagsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([Cell::new("FLAG"), Cell::new("PERMANENT")]));

        for entry in self.build_entries() {
            table.add_row(Row::from([
                Cell::new(&entry.name),
                Cell::new(if entry.permanent { "true" } else { "" }),
            ]));
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for FlagsTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.build_entries().serialize(serializer)
    }
}
