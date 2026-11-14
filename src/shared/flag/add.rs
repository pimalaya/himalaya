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
    error::{ItemError, partial_when},
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

        let outcome = client.store_flags(&mailbox, &ids, &flags, FlagOp::Add)?;

        let output = FlagAddOutput {
            flags: client.written_flags(flags),
            count: outcome.value,
            queued: client.queues_writes(),
            errors: outcome.errors,
        };

        printer.out(&output)?;
        partial_when(&output.errors)
    }
}

/// The `flag add` output, naming the flags that were added.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlagAddOutput {
    /// The flags written.
    flags: Vec<Flag>,
    /// How many of the given messages were acted on.
    count: usize,
    /// Whether a sync engine applies the write later rather than now.
    queued: bool,
    /// The error of each message not acted on, in the order given.
    errors: Vec<ItemError>,
}

impl fmt::Display for FlagAddOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.count;
        let flags: Vec<&str> = self.flags.iter().map(Flag::raw).collect();
        let flags = flags.join(", ");
        if self.queued {
            writeln!(f, "Flags queued for adding on {count} message(s): {flags}")?;
        } else {
            writeln!(f, "Successfully added flags to {count} message(s): {flags}")?;
        }

        for error in &self.errors {
            writeln!(f, "Error:  {error}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        email::flag::{Flag, IanaFlag},
        error::missing_message,
        shared::flag::add::FlagAddOutput,
    };

    /// One id of two found: the sentence counts it, then the unknown one
    /// prints on its `Error:` line.
    #[test]
    fn an_unknown_id_prints_after_the_sentence() {
        let output = FlagAddOutput {
            flags: vec![Flag::from_iana(IanaFlag::Flagged)],
            count: 1,
            queued: false,
            errors: vec![missing_message("99999999")],
        };

        assert_eq!(
            output.to_string(),
            "Successfully added flags to 1 message(s): \\Flagged\n\
             Error:  99999999  message-not-found  Message not found in the mailbox\n"
        );
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            r#"{"flags":[{"raw":"\\Flagged","iana":"flagged"}],"count":1,"queued":false,"errors":[{"id":"99999999","code":"message-not-found","error":"Message not found in the mailbox","sources":[]}]}"#
        );
    }

    #[test]
    fn output_counts_the_messages_and_ends_the_line() {
        let output = FlagAddOutput {
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
            "Successfully added flags to 2 message(s): \\Seen, \\Flagged\n"
        );
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            r#"{"flags":[{"raw":"\\Seen","iana":"seen"},{"raw":"\\Flagged","iana":"flagged"}],"count":2,"queued":false,"errors":[]}"#
        );
    }

    #[test]
    fn a_queued_change_says_it_is_queued() {
        let output = FlagAddOutput {
            flags: vec![Flag::from_iana(IanaFlag::Seen)],
            count: 1,
            queued: true,
            errors: Vec::new(),
        };

        assert_eq!(
            output.to_string(),
            "Flags queued for adding on 1 message(s): \\Seen\n"
        );
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            r#"{"flags":[{"raw":"\\Seen","iana":"seen"}],"count":1,"queued":true,"errors":[]}"#
        );
    }
}
