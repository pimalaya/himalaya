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

use crate::{
    account::context::Account,
    email::flag::{Flag, IanaFlag},
};

/// List the standard Maildir flags.
///
/// Displays the six standard Maildir info flags with their
/// single-letter codes (D, F, P, R, S, T).
#[derive(Debug, Parser)]
pub struct MaildirFlagListCommand;

impl MaildirFlagListCommand {
    /// Lists the six standard flags and their info letters.
    pub fn execute(self, printer: &mut impl Printer, _account: &mut Account) -> Result<()> {
        let standard = [
            (MaildirFlag::Draft, IanaFlag::Draft),
            (MaildirFlag::Flagged, IanaFlag::Flagged),
            (MaildirFlag::Passed, IanaFlag::Forwarded),
            (MaildirFlag::Replied, IanaFlag::Answered),
            (MaildirFlag::Seen, IanaFlag::Seen),
            (MaildirFlag::Trashed, IanaFlag::Deleted),
        ];

        printer.out(MaildirFlagListOutput {
            flags: standard
                .iter()
                .map(|(flag, iana)| Flag::spelled(flag.to_string(), Some(*iana)))
                .collect(),
            names: standard
                .iter()
                .map(|(flag, _)| format!("{flag:?}"))
                .collect(),
        })
    }
}

/// The `maildir flag list` output, the standard Maildir flags as the
/// shared flags print them, each info letter as `raw`.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaildirFlagListOutput {
    /// The standard flags, each info letter as `raw`.
    flags: Vec<Flag>,
    /// The Maildir name of each flag, for the text form.
    #[serde(skip)]
    names: Vec<String>,
}

impl fmt::Display for MaildirFlagListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::default();

        for (flag, name) in self.flags.iter().zip(&self.names) {
            columns.push([flag.raw(), name]);
        }

        write!(f, "{columns}")
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::{account::context::Account, maildir::flag::list::MaildirFlagListCommand};

    /// A Maildir flag prints as every flag does, `{raw, iana}`, its letter
    /// as `raw`.
    #[test]
    fn a_letter_prints_as_the_shared_flag() {
        let mut printer = CapturePrinter::default();
        MaildirFlagListCommand
            .execute(&mut printer, &mut Account::default())
            .unwrap();
        let json = printer.into_single().unwrap();

        assert_eq!(json["flags"][0], json!({"raw": "D", "iana": "draft"}));
        assert_eq!(json["flags"].as_array().unwrap().len(), 6);
    }
}
