//! # mbox flag list
//!
//! The `mbox flag list` command, naming the six mbox flags, their
//! letters and the header carrying them.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{columns::Columns, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::flag::{Flag, IanaFlag},
};

/// List the mbox flags.
///
/// Displays the six flags the `Status` and `X-Status` headers carry,
/// with their single-letter codes.
#[derive(Debug, Parser)]
pub struct MboxFlagListCommand;

impl MboxFlagListCommand {
    /// Lists the six flags, their letters and headers.
    pub fn execute(self, printer: &mut impl Printer, _account: &mut Account) -> Result<()> {
        let flag = |raw: &str, header: &str, name: &str, iana| MboxFlagRow {
            flag: Flag::spelled(raw, iana),
            header: header.into(),
            name: name.into(),
        };

        printer.out(MboxFlagListOutput {
            flags: vec![
                flag("R", "Status", "Seen", Some(IanaFlag::Seen)),
                flag("O", "Status", "Old", None),
                flag("A", "X-Status", "Answered", Some(IanaFlag::Answered)),
                flag("F", "X-Status", "Flagged", Some(IanaFlag::Flagged)),
                flag("T", "X-Status", "Draft", Some(IanaFlag::Draft)),
                flag("D", "X-Status", "Deleted", Some(IanaFlag::Deleted)),
            ],
        })
    }
}

/// The `mbox flag list` output, the mbox flags.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MboxFlagListOutput {
    flags: Vec<MboxFlagRow>,
}

/// One mbox flag: the flag, as the shared flags print it, its letter as
/// `raw` and `iana` `null` for `O`, which has none, and the header
/// carrying the letter.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MboxFlagRow {
    #[serde(flatten)]
    flag: Flag,
    /// The header carrying the letter.
    header: String,
    /// The name of the flag, for the text form.
    #[serde(skip)]
    name: String,
}

impl fmt::Display for MboxFlagListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::default();

        for flag in &self.flags {
            columns.push([flag.flag.raw(), &flag.header, &flag.name]);
        }

        write!(f, "{columns}")
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::{account::context::Account, mbox::flag::list::MboxFlagListCommand};

    /// An mbox flag prints as every flag does, `{raw, iana}`, beside the
    /// header carrying its letter.
    #[test]
    fn a_letter_prints_as_the_shared_flag_with_its_header() {
        let mut printer = CapturePrinter::default();
        MboxFlagListCommand
            .execute(&mut printer, &mut Account::default())
            .unwrap();
        let json = printer.into_single().unwrap();

        let old = json!({"raw": "O", "iana": null, "header": "Status"});
        assert_eq!(json["flags"][1], old);
        let seen = json!({"raw": "R", "iana": "seen", "header": "Status"});
        assert_eq!(json["flags"][0], seen);
    }
}
