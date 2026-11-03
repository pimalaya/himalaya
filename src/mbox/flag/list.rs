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

use crate::{account::context::Account, email::flag::IanaFlag, shared::table::style_from_preset};

/// List the mbox flags.
///
/// Displays the six flags the `Status` and `X-Status` headers carry,
/// with their single-letter codes.
#[derive(Debug, Parser)]
pub struct MboxFlagListCommand;

impl MboxFlagListCommand {
    /// Tables the six flags, their letters and headers.
    pub fn execute(self, printer: &mut impl Printer, account: &mut Account) -> Result<()> {
        let flag = |raw: &str, header: &str, name: &str, iana| FlagRow {
            raw: raw.into(),
            iana,
            header: header.into(),
            name: name.into(),
        };

        let table = MboxFlagListOutput {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            flags: vec![
                flag("R", "Status", "Seen", Some(IanaFlag::Seen)),
                flag("O", "Status", "Old", None),
                flag("A", "X-Status", "Answered", Some(IanaFlag::Answered)),
                flag("F", "X-Status", "Flagged", Some(IanaFlag::Flagged)),
                flag("T", "X-Status", "Draft", Some(IanaFlag::Draft)),
                flag("D", "X-Status", "Deleted", Some(IanaFlag::Deleted)),
            ],
        };

        printer.out(table)
    }
}

/// Renderable table of the mbox flags.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MboxFlagListOutput {
    #[serde(skip_serializing)]
    #[schemars(skip)]
    preset: String,
    #[serde(skip_serializing)]
    #[schemars(skip)]
    arrangement: ContentArrangement,
    flags: Vec<FlagRow>,
}

/// One row of the mbox flags table: the flag, as the shared flags print
/// it, its letter as `raw`, and the header carrying the letter.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FlagRow {
    /// The letter.
    raw: String,
    /// The IANA keyword the letter stands for, `null` for `O`, which
    /// has none.
    iana: Option<IanaFlag>,
    /// The header carrying the letter.
    header: String,
    /// The name of the flag, for the text form.
    #[serde(skip)]
    name: String,
}

impl fmt::Display for MboxFlagListOutput {
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
                Cell::new(&flag.raw),
                Cell::new(&flag.header),
                Cell::new(&flag.name),
            ]));
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
