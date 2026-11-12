//! # pimdir flag list
//!
//! The `pimdir flag list` command, naming the flags one message carries.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use log::warn;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, backend::Backend, listing::Listing, pimdir::client::PimdirClient,
    shared::mailbox::arg::MailboxArg,
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

        printer.out(PimdirFlagListOutput {
            account: account.name().to_string(),
            mailbox,
            message: self.id,
            flags,
        })
    }
}

/// The `pimdir flag list` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirFlagListOutput {
    /// The name of the account read.
    #[serde(skip)]
    account: String,
    /// The mailbox of the message, its alias resolved.
    #[serde(skip)]
    mailbox: String,
    /// The id of the message.
    #[serde(skip)]
    message: String,
    /// The raw spellings, sorted, or `null` when the sync engine never
    /// read the message's flags.
    flags: Option<Vec<String>>,
}

impl fmt::Display for PimdirFlagListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Pimdir));
        listing.head("Mailbox", &self.mailbox);
        listing.head("Message", &self.message);

        for flag in self.flags.iter().flatten() {
            listing.items.push([sanitize::text(flag)]);
        }

        write!(f, "{listing}")
    }
}

#[cfg(test)]
mod tests {
    use crate::pimdir::flag::list::PimdirFlagListOutput;

    fn list(flags: Option<Vec<String>>) -> PimdirFlagListOutput {
        PimdirFlagListOutput {
            account: String::from("work"),
            mailbox: String::from("INBOX"),
            message: String::from("7"),
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
    fn the_list_names_one_flag_per_line() {
        let head = "Account:    work (pimdir)\nMailbox:    INBOX\nMessage:    7\n";

        let known = list(Some(vec!["\\Seen".into(), "act".into()]));
        assert_eq!(known.to_string(), format!("{head}\n\\Seen\nact\n"));
        assert_eq!(list(None).to_string(), head);
    }
}
