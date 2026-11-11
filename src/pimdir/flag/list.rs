//! # pimdir flag list
//!
//! The `pimdir flag list` command, naming the flags one message carries.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use log::warn;
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, pimdir::client::PimdirClient, shared::mailbox::arg::MailboxArg,
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
    /// Reads the message's flags and lists them.
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

        printer.out(PimdirFlagListOutput { flags })
    }
}

/// The `pimdir flag list` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirFlagListOutput {
    /// The raw spellings, sorted, or `null` when the sync engine never
    /// read the message's flags.
    flags: Option<Vec<String>>,
}

impl fmt::Display for PimdirFlagListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::new(["FLAG"]);

        for flag in self.flags.iter().flatten() {
            columns.push([sanitize::text(flag)]);
        }

        write!(f, "{columns}")
    }
}

#[cfg(test)]
mod tests {
    use crate::pimdir::flag::list::PimdirFlagListOutput;

    fn list(flags: Option<Vec<String>>) -> PimdirFlagListOutput {
        PimdirFlagListOutput { flags }
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
    fn the_list_names_one_flag_per_row() {
        let known = list(Some(vec!["\\Seen".into(), "act".into()]));
        assert_eq!(known.to_string(), "FLAG\n\\Seen\nact\n");
        assert_eq!(list(None).to_string(), "FLAG\n");
    }
}
