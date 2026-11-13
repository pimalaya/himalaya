//! # Flag add
//!
//! The `flag add` command, adding flags to the existing set of one or
//! more messages.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::flag::{Flag, FlagOp},
    error::message_not_found,
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
        let ids = self.message_ids.ids();
        let flags: Vec<Flag> = self.flags.inner.iter().map(Into::into).collect();

        let count = client.store_flags(&mailbox, &ids, &flags, FlagOp::Add)?;
        if count == 0 {
            return Err(message_not_found(
                "No flag added: no id matched in the mailbox",
            ));
        }

        let queued = client.queues_writes();
        printer.out(FlagAddOutput {
            flags: client.written_flags(flags),
            count,
            queued,
        })
    }
}

/// The `flag add` output, naming the flags that were added.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlagAddOutput {
    /// The flags written.
    flags: Vec<Flag>,
    /// How many of the given messages were found.
    count: usize,
    /// Whether a sync engine applies the write later rather than now.
    queued: bool,
}

impl fmt::Display for FlagAddOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.count;
        let flags: Vec<&str> = self.flags.iter().map(Flag::raw).collect();
        let flags = flags.join(", ");
        if self.queued {
            return writeln!(f, "Flags queued for adding on {count} message(s): {flags}");
        }
        writeln!(f, "Successfully added flags to {count} message(s): {flags}")
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        email::flag::{Flag, IanaFlag},
        shared::flag::add::FlagAddOutput,
    };

    #[test]
    fn output_counts_the_messages_and_ends_the_line() {
        let output = FlagAddOutput {
            flags: vec![
                Flag::from_iana(IanaFlag::Seen),
                Flag::from_iana(IanaFlag::Flagged),
            ],
            count: 2,
            queued: false,
        };

        assert_eq!(
            output.to_string(),
            "Successfully added flags to 2 message(s): \\Seen, \\Flagged\n"
        );
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            r#"{"flags":[{"raw":"\\Seen","iana":"seen"},{"raw":"\\Flagged","iana":"flagged"}],"count":2,"queued":false}"#
        );
    }

    #[test]
    fn a_queued_change_says_it_is_queued() {
        let output = FlagAddOutput {
            flags: vec![Flag::from_iana(IanaFlag::Seen)],
            count: 1,
            queued: true,
        };

        assert_eq!(
            output.to_string(),
            "Flags queued for adding on 1 message(s): \\Seen\n"
        );
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            r#"{"flags":[{"raw":"\\Seen","iana":"seen"}],"count":1,"queued":true}"#
        );
    }
}
