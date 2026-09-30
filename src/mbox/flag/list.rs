//! # mbox flag list
//!
//! The `mbox flag list` command, naming the six mbox flags, their
//! letters and the header carrying them.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{
    printer::Printer,
    table::{Cell, ContentArrangement, Row, Table},
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, shared::table::style_from_preset};

/// List the mbox flags.
///
/// Displays the six flags the `Status` and `X-Status` headers carry,
/// with their single-letter codes.
#[derive(Debug, Parser)]
pub struct MboxFlagListCommand;

impl MboxFlagListCommand {
    /// Tables the six flags, their letters and headers.
    pub fn execute(self, printer: &mut impl Printer, account: &mut Account) -> Result<()> {
        let flag = |code: &str, header: &str, name: &str| FlagRow {
            code: code.into(),
            header: header.into(),
            name: name.into(),
        };

        let table = FlagsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            flags: vec![
                flag("R", "Status", "Seen"),
                flag("O", "Status", "Old"),
                flag("A", "X-Status", "Answered"),
                flag("F", "X-Status", "Flagged"),
                flag("T", "X-Status", "Draft"),
                flag("D", "X-Status", "Deleted"),
            ],
        };

        printer.out(table)
    }
}

/// Renderable table of the mbox flags.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct FlagsTable {
    #[serde(skip_serializing)]
    #[schemars(skip)]
    preset: String,
    #[serde(skip_serializing)]
    #[schemars(skip)]
    arrangement: ContentArrangement,
    flags: Vec<FlagRow>,
}

/// One row of the mbox flags table: letter, header and name.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct FlagRow {
    code: String,
    header: String,
    name: String,
}

impl fmt::Display for FlagsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("CODE"),
                Cell::new("HEADER"),
                Cell::new("NAME"),
            ]));

        for flag in &self.flags {
            table.add_row(Row::from([
                Cell::new(&flag.code),
                Cell::new(&flag.header),
                Cell::new(&flag.name),
            ]));
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
