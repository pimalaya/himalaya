//! # Vacation get
//!
//! The `vacation get` command, printing the account's auto-reply.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::vacation::Vacation,
    shared::{message::read::sanitize_body, vacation::cli::VacationClient},
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
        _account: &mut Account,
        client: VacationClient<'_>,
    ) -> Result<()> {
        printer.out(VacationGetOutput {
            vacation: client.get()?,
        })
    }
}

/// The `vacation get` output, the auto-reply itself.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct VacationGetOutput {
    /// The auto-reply.
    pub vacation: Vacation,
}

impl fmt::Display for VacationGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", vacation_fields(&self.vacation))
    }
}

/// The auto-reply as a record, the unset values left out and the
/// single-line ones made printable; the bodies keep their lines.
pub fn vacation_fields(vacation: &Vacation) -> Fields<'static> {
    let mut fields = Fields::default();

    let enabled = if vacation.enabled { "yes" } else { "no" };
    fields.push("Enabled", enabled);

    let rows = [
        ("From", vacation.from.as_deref().map(line)),
        ("To", vacation.to.as_deref().map(line)),
        ("Subject", vacation.subject.as_deref().map(line)),
        ("Text", vacation.text.as_deref().map(sanitize_body)),
        ("HTML", vacation.html.as_deref().map(sanitize_body)),
    ];

    for (label, value) in rows {
        if let Some(value) = value {
            fields.push(label, value);
        }
    }

    fields
}

/// A single-line value made printable.
fn line(value: &str) -> String {
    sanitize::text(value).into_owned()
}
