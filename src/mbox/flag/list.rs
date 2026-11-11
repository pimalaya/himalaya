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

use crate::{account::context::Account, email::flag::IanaFlag};

/// List the mbox flags.
///
/// Displays the six flags the `Status` and `X-Status` headers carry,
/// with their single-letter codes.
#[derive(Debug, Parser)]
pub struct MboxFlagListCommand;

impl MboxFlagListCommand {
    /// Lists the six flags, their letters and headers.
    pub fn execute(self, printer: &mut impl Printer, _account: &mut Account) -> Result<()> {
        let flag = |raw: &str, header: &str, name: &str, iana| FlagRow {
            raw: raw.into(),
            iana,
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
    flags: Vec<FlagRow>,
}

/// One mbox flag: the flag, as the shared flags print it, its letter as
/// `raw`, and the header carrying the letter.
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
        let mut columns = Columns::new(["CODE", "HEADER", "NAME"]);

        for flag in &self.flags {
            columns.push([&flag.raw, &flag.header, &flag.name]);
        }

        write!(f, "{columns}")
    }
}
