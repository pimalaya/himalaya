//! # Maildir flag list
//!
//! The `maildir flag list` command, naming the six standard flags and
//! their letters.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_maildir::flag::MaildirFlag;
use pimalaya_cli::printer::Printer;
use pimalaya_cli::table::{Cell, ContentArrangement, Row, Table};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, email::flag::IanaFlag, shared::table::style_from_preset};

/// List the standard Maildir flags.
///
/// Displays the six standard Maildir info flags with their
/// single-letter codes (P, R, S, T, D, F).
#[derive(Debug, Parser)]
pub struct MaildirFlagListCommand;

impl MaildirFlagListCommand {
    /// Tables the six standard flags and their info letters.
    pub fn execute(self, printer: &mut impl Printer, account: &mut Account) -> Result<()> {
        let table = MaildirFlagListOutput {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            flags: vec![
                FlagRow::new(MaildirFlag::Passed, IanaFlag::Forwarded),
                FlagRow::new(MaildirFlag::Replied, IanaFlag::Answered),
                FlagRow::new(MaildirFlag::Seen, IanaFlag::Seen),
                FlagRow::new(MaildirFlag::Trashed, IanaFlag::Deleted),
                FlagRow::new(MaildirFlag::Draft, IanaFlag::Draft),
                FlagRow::new(MaildirFlag::Flagged, IanaFlag::Flagged),
            ],
        };

        printer.out(table)
    }
}

/// Renderable table of the standard Maildir flags.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaildirFlagListOutput {
    #[serde(skip_serializing)]
    #[schemars(skip)]
    preset: String,
    #[serde(skip_serializing)]
    #[schemars(skip)]
    arrangement: ContentArrangement,
    flags: Vec<FlagRow>,
}

/// One row of the Maildir flags table: the flag, as the shared flags
/// print it, its info letter as `raw`.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FlagRow {
    /// The info letter.
    raw: String,
    /// The IANA keyword the letter stands for.
    iana: IanaFlag,
    /// The Maildir name of the flag, for the text form.
    #[serde(skip)]
    name: String,
}

impl FlagRow {
    pub fn new(flag: MaildirFlag, iana: IanaFlag) -> Self {
        Self {
            raw: flag.to_string(),
            iana,
            name: format!("{flag:?}"),
        }
    }
}

impl fmt::Display for MaildirFlagListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("CODE"), Cell::new("NAME")]));

        for flag in &self.flags {
            table.add_row(Row::from([Cell::new(&flag.raw), Cell::new(&flag.name)]));
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
