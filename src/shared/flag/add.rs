//! # Flag add
//!
//! The `flag add` command, adding flags to the existing set of one or
//! more messages.

use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::flag::{Flag, FlagOp},
    shared::{
        client::EmailClient,
        flag::arg::{FlagsArg, MessageIdsArg},
        mailbox::arg::MailboxArg,
    },
};

/// Add flags to messages, keeping the ones already set.
#[derive(Debug, Parser)]
pub struct FlagAddCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    #[command(flatten)]
    pub message_ids: MessageIdsArg,
    #[command(flatten)]
    pub flags: FlagsArg,
}

impl FlagAddCommand {
    /// Adds the flags and reports which ones landed.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account);
        let ids: Vec<&str> = self.message_ids.inner.iter().map(String::as_str).collect();
        let flags: Vec<Flag> = self.flags.inner.iter().map(Into::into).collect();

        let count = client.store_flags(&mailbox, &ids, &flags, FlagOp::Add)?;
        if count == 0 {
            bail!("No flag added: no id matched in the mailbox");
        }

        let flags: Vec<String> = self.flags.inner.iter().map(ToString::to_string).collect();
        printer.out(FlagAddOutput { flags, count })
    }
}

/// The `flag add` output, naming the flags that were added.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlagAddOutput {
    flags: Vec<String>,
    /// How many of the given messages were found.
    count: usize,
}

impl fmt::Display for FlagAddOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (count, flags) = (self.count, self.flags.join(", "));
        writeln!(f, "Successfully added flags to {count} message(s): {flags}")
    }
}

#[cfg(test)]
mod tests {
    use super::FlagAddOutput;

    #[test]
    fn output_counts_the_messages_and_ends_the_line() {
        let output = FlagAddOutput {
            flags: vec!["seen".into(), "flagged".into()],
            count: 2,
        };

        assert_eq!(
            output.to_string(),
            "Successfully added flags to 2 message(s): seen, flagged\n"
        );
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            r#"{"flags":["seen","flagged"],"count":2}"#
        );
    }
}
