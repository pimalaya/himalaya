//! # Vacation set
//!
//! The `vacation set` command, changing the account's auto-reply.

use std::fmt;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::vacation::{Vacation, VacationPatch, parse_date},
    shared::vacation::{cli::VacationClient, get::vacation_table},
};

/// Change the auto-reply, then print it as it stands.
///
/// Only what is given changes, the rest kept as it is. The auto-reply
/// itself is turned on with `--enable` and off with `--disable`, never by
/// accident. Once on, it answers incoming mail, so an agent reaching this
/// command can make the account send mail.
#[derive(Debug, Parser)]
pub struct VacationSetCommand {
    /// Turn the auto-reply on.
    #[arg(long, conflicts_with = "disable")]
    pub enable: bool,
    /// Turn the auto-reply off.
    #[arg(long)]
    pub disable: bool,
    /// When it starts answering: RFC 3339, or YYYY-MM-DD for midnight
    /// UTC.
    #[arg(long, value_name = "DATE", value_parser = parse_date)]
    pub from: Option<DateTime<Utc>>,
    /// When it stops answering: RFC 3339, or YYYY-MM-DD for midnight UTC.
    #[arg(long, value_name = "DATE", value_parser = parse_date)]
    pub to: Option<DateTime<Utc>>,
    /// The subject of the reply.
    #[arg(long, value_name = "TEXT")]
    pub subject: Option<String>,
    /// The plain text body of the reply.
    #[arg(long, value_name = "TEXT")]
    pub text: Option<String>,
    /// The HTML body of the reply.
    #[arg(long, value_name = "HTML")]
    pub html: Option<String>,
}

impl VacationSetCommand {
    /// Applies the changes and prints the auto-reply as it then stands.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: VacationClient<'_>,
    ) -> Result<()> {
        let enabled = match (self.enable, self.disable) {
            (true, _) => Some(true),
            (_, true) => Some(false),
            _ => None,
        };

        let patch = VacationPatch {
            enabled,
            from: self.from,
            to: self.to,
            subject: self.subject,
            text: self.text,
            html: self.html,
        };

        printer.out(VacationSetOutput {
            preset: account.table_preset().to_string(),
            vacation: client.set(patch)?,
        })
    }
}

/// The `vacation set` output, the auto-reply as it stands once changed.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct VacationSetOutput {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The auto-reply.
    pub vacation: Vacation,
}

impl fmt::Display for VacationSetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        writeln!(f, "{}", vacation_table(&self.preset, &self.vacation))
    }
}
