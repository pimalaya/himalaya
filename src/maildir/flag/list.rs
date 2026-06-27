use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_maildir::flag::types::MaildirFlag;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;

/// List the standard Maildir flags.
///
/// Displays the six standard Maildir info flags with their
/// single-letter codes (P, R, S, T, D, F).
#[derive(Debug, Parser)]
pub struct MaildirFlagListCommand;

impl MaildirFlagListCommand {
    pub fn execute(self, printer: &mut impl Printer, account: &mut Account) -> Result<()> {
        let table = FlagsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            flags: vec![
                FlagRow::new(MaildirFlag::Passed),
                FlagRow::new(MaildirFlag::Replied),
                FlagRow::new(MaildirFlag::Seen),
                FlagRow::new(MaildirFlag::Trashed),
                FlagRow::new(MaildirFlag::Draft),
                FlagRow::new(MaildirFlag::Flagged),
            ],
        };

        printer.out(table)
    }
}

/// Renderable table of the standard Maildir flags.
#[derive(Clone, Debug, Serialize)]
pub struct FlagsTable {
    #[serde(skip_serializing)]
    preset: String,
    #[serde(skip_serializing)]
    arrangement: ContentArrangement,
    flags: Vec<FlagRow>,
}

/// One row of the Maildir flags table: code and name.
#[derive(Clone, Debug, Serialize)]
pub struct FlagRow {
    code: String,
    name: String,
}

impl FlagRow {
    pub fn new(flag: MaildirFlag) -> Self {
        Self {
            code: flag.to_string(),
            name: format!("{flag:?}"),
        }
    }
}

impl fmt::Display for FlagsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("CODE"), Cell::new("NAME")]));

        for flag in &self.flags {
            table.add_row(Row::from([Cell::new(&flag.code), Cell::new(&flag.name)]));
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
