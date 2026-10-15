//! # pimdir flag list
//!
//! The `pimdir flag list` command, naming the flags one message carries.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use log::warn;
use pimalaya_cli::printer::Printer;
use pimalaya_cli::table::{Cell, ContentArrangement, Row, Table};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    pimdir::client::PimdirClient,
    shared::{mailbox::arg::MailboxArg, table::style_from_preset},
};

/// List the flags a pimdir message carries, one raw spelling per row.
///
/// Changes staged and not yet synced are included, as every pimdir read
/// shows them. A message whose flags the sync engine never read lists
/// none, and `--json` prints `null` for it rather than an empty list.
#[derive(Debug, Parser)]
pub struct PimdirFlagListCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// The message's id.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl PimdirFlagListCommand {
    /// Reads the message's flags and tables them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account);
        let flags = client.read_flags(&mailbox, &self.id)?;
        if flags.is_none() {
            warn!(
                "the flags of `{}` were never read by the sync engine",
                self.id
            );
        }

        printer.out(PimdirFlagList {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            flags,
        })
    }
}

/// The `pimdir flag list` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct PimdirFlagList {
    #[serde(skip_serializing)]
    #[schemars(skip)]
    preset: String,
    #[serde(skip_serializing)]
    #[schemars(skip)]
    arrangement: ContentArrangement,
    /// The raw spellings, sorted, or `null` when the sync engine never
    /// read the message's flags.
    flags: Option<Vec<String>>,
}

impl fmt::Display for PimdirFlagList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("FLAG")]));

        for flag in self.flags.iter().flatten() {
            table.add_row(Row::from([Cell::new(flag)]));
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(flags: Option<Vec<String>>) -> PimdirFlagList {
        PimdirFlagList {
            preset: String::new(),
            arrangement: ContentArrangement::Disabled,
            flags,
        }
    }

    #[test]
    fn flags_serialize_as_a_list_and_unread_ones_as_null() {
        let known = list(Some(vec!["\\Seen".into(), "act".into()]));
        assert_eq!(
            serde_json::to_string(&known).unwrap(),
            r#"{"flags":["\\Seen","act"]}"#
        );
        assert_eq!(
            serde_json::to_string(&list(None)).unwrap(),
            r#"{"flags":null}"#
        );
    }

    #[test]
    fn the_table_lists_one_flag_per_row() {
        let table = list(Some(vec!["\\Seen".into(), "act".into()])).to_string();
        assert!(table.contains("FLAG"), "{table}");
        assert!(table.contains("\\Seen") && table.contains("act"), "{table}");
        assert!(!list(None).to_string().contains("act"));
    }
}
