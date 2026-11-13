//! # Maildir flag list
//!
//! The `maildir flag list` command, naming the six standard flags and
//! their letters.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_maildir::flag::MaildirFlag;
use pimalaya_cli::{columns::Columns, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, email::flag::IanaFlag};

/// List the standard Maildir flags.
///
/// Displays the six standard Maildir info flags with their
/// single-letter codes (D, F, P, R, S, T).
#[derive(Debug, Parser)]
pub struct MaildirFlagListCommand;

impl MaildirFlagListCommand {
    /// Lists the six standard flags and their info letters.
    pub fn execute(self, printer: &mut impl Printer, _account: &mut Account) -> Result<()> {
        let table = MaildirFlagListOutput {
            flags: vec![
                FlagRow::new(MaildirFlag::Draft, IanaFlag::Draft),
                FlagRow::new(MaildirFlag::Flagged, IanaFlag::Flagged),
                FlagRow::new(MaildirFlag::Passed, IanaFlag::Forwarded),
                FlagRow::new(MaildirFlag::Replied, IanaFlag::Answered),
                FlagRow::new(MaildirFlag::Seen, IanaFlag::Seen),
                FlagRow::new(MaildirFlag::Trashed, IanaFlag::Deleted),
            ],
        };

        printer.out(table)
    }
}

/// The `maildir flag list` output, the standard Maildir flags.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaildirFlagListOutput {
    flags: Vec<FlagRow>,
}

/// One Maildir flag: the flag, as the shared flags print it, its info
/// letter as `raw`.
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
        let mut columns = Columns::default();

        for flag in &self.flags {
            columns.push([&flag.raw, &flag.name]);
        }

        write!(f, "{columns}")
    }
}
