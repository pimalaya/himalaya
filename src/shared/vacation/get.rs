//! # Vacation get
//!
//! The `vacation get` command, printing the account's auto-reply.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{
    printer::Printer,
    table::{Cell, Row, Table, sanitize},
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::vacation::Vacation,
    shared::{
        message::read::sanitize_body, table::style_from_preset, vacation::cli::VacationClient,
    },
};

/// Print the auto-reply.
///
/// A backend holding none prints it off, with nothing set.
#[derive(Debug, Parser)]
pub struct VacationGetCommand;

impl VacationGetCommand {
    /// Reads the auto-reply and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: VacationClient<'_>,
    ) -> Result<()> {
        printer.out(VacationGetOutput {
            preset: account.table_preset().to_string(),
            vacation: client.get()?,
        })
    }
}

/// The `vacation get` output, the auto-reply itself.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct VacationGetOutput {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The auto-reply.
    pub vacation: Vacation,
}

impl fmt::Display for VacationGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        writeln!(f, "{}", vacation_table(&self.preset, &self.vacation))
    }
}

/// The auto-reply as a key and value table, the unset values left out
/// and the single-line ones made printable; the bodies keep their lines.
pub fn vacation_table(preset: &str, vacation: &Vacation) -> Table {
    let mut table = Table::new();

    table
        .load_style(style_from_preset(preset))
        .set_header(Row::from([Cell::new("KEY"), Cell::new("VALUE")]));

    let enabled = if vacation.enabled { "yes" } else { "no" };
    table.add_row(Row::from([Cell::new("Enabled"), Cell::new(enabled)]));

    let rows = [
        ("From", vacation.from.as_deref().map(line)),
        ("To", vacation.to.as_deref().map(line)),
        ("Subject", vacation.subject.as_deref().map(line)),
        ("Text", vacation.text.as_deref().map(sanitize_body)),
        ("HTML", vacation.html.as_deref().map(sanitize_body)),
    ];

    for (key, value) in rows {
        if let Some(value) = value {
            table.add_row(Row::from([Cell::new(key), Cell::new(value)]));
        }
    }

    table
}

/// A single-line value made printable.
fn line(value: &str) -> String {
    sanitize(value).into_owned()
}
