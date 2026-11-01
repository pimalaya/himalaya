//! # Flag remove
//!
//! The `flag remove` command, removing flags from the existing set of one
//! or more messages.

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

/// Remove flags from messages, keeping the ones not named.
#[derive(Debug, Parser)]
pub struct FlagRemoveCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    #[command(flatten)]
    pub message_ids: MessageIdsArg,
    #[command(flatten)]
    pub flags: FlagsArg,
}

impl FlagRemoveCommand {
    /// Removes the flags and reports which ones went.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account);
        let ids: Vec<&str> = self.message_ids.inner.iter().map(String::as_str).collect();
        let flags: Vec<Flag> = self.flags.inner.iter().map(Into::into).collect();

        let count = client.store_flags(&mailbox, &ids, &flags, FlagOp::Remove)?;
        if count == 0 {
            bail!("No flag removed: no id matched in the mailbox");
        }

        let flags: Vec<String> = self.flags.inner.iter().map(ToString::to_string).collect();
        let queued = client.queues_writes();
        printer.out(FlagRemoveOutput {
            flags,
            count,
            queued,
        })
    }
}

/// The `flag remove` output, naming the flags that were removed.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlagRemoveOutput {
    flags: Vec<String>,
    /// How many of the given messages were found.
    count: usize,
    /// Whether the backend queued the change for its sync engine rather
    /// than made it, which only the wording says.
    #[serde(skip)]
    queued: bool,
}

impl fmt::Display for FlagRemoveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (count, flags) = (self.count, self.flags.join(", "));
        if self.queued {
            return writeln!(
                f,
                "Flags queued for removal from {count} message(s): {flags}"
            );
        }
        writeln!(
            f,
            "Successfully removed flags from {count} message(s): {flags}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::FlagRemoveOutput;

    #[test]
    fn output_counts_the_messages_and_ends_the_line() {
        let output = FlagRemoveOutput {
            flags: vec!["seen".into(), "flagged".into()],
            count: 2,
            queued: false,
        };

        assert_eq!(
            output.to_string(),
            "Successfully removed flags from 2 message(s): seen, flagged\n"
        );
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            r#"{"flags":["seen","flagged"],"count":2}"#
        );
    }
}
