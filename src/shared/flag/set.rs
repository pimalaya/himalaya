//! # Flag set
//!
//! The `flag set` command, replacing the whole flag set of one or more
//! messages.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::flag::{Flag, FlagOp},
    error::{ItemError, partial_when, write_item_errors},
    shared::{
        client::EmailClient,
        flag::arg::{FlagsArg, MessageIdsArg},
        mailbox::arg::MailboxArg,
    },
};

/// Replace the flags of messages, dropping the ones not named.
#[derive(Debug, Parser)]
pub struct FlagSetCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    #[command(flatten)]
    pub message_ids: MessageIdsArg,
    #[command(flatten)]
    pub flags: FlagsArg,
}

impl FlagSetCommand {
    /// Replaces the flags and reports which ones landed.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account);
        let ids = self.message_ids.ids();
        let flags: Vec<Flag> = self.flags.inner.iter().map(Into::into).collect();

        let outcome = client.store_flags(&mailbox, &ids, &flags, FlagOp::Set)?;

        let output = FlagSetOutput {
            flags: client.written_flags(flags),
            count: outcome.value,
            queued: client.queues_writes(),
            errors: outcome.errors,
        };

        printer.out(&output)?;
        partial_when(&output.errors)
    }
}

/// The `flag set` output, naming the flags that were set.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlagSetOutput {
    /// The flags written.
    flags: Vec<Flag>,
    /// How many of the given messages were acted on.
    count: usize,
    /// Whether a sync engine applies the write later rather than now.
    queued: bool,
    /// The error of each message not acted on, in the order given.
    errors: Vec<ItemError>,
}

impl fmt::Display for FlagSetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.count;
        let flags: Vec<&str> = self.flags.iter().map(Flag::raw).collect();
        let flags = flags.join(", ");
        if self.queued {
            writeln!(f, "Flags queued for setting on {count} message(s): {flags}")?;
        } else {
            writeln!(f, "Successfully set flags on {count} message(s): {flags}")?;
        }

        write_item_errors(f, &self.errors)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        email::flag::{Flag, IanaFlag},
        shared::flag::set::FlagSetOutput,
    };

    #[test]
    fn output_counts_the_messages_and_ends_the_line() {
        let output = FlagSetOutput {
            flags: vec![
                Flag::from_iana(IanaFlag::Seen),
                Flag::from_iana(IanaFlag::Flagged),
            ],
            count: 2,
            queued: false,
            errors: Vec::new(),
        };

        assert_eq!(
            output.to_string(),
            "Successfully set flags on 2 message(s): \\Seen, \\Flagged\n"
        );
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            r#"{"flags":[{"raw":"\\Seen","iana":"seen"},{"raw":"\\Flagged","iana":"flagged"}],"count":2,"queued":false,"errors":[]}"#
        );
    }
}
