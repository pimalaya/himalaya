//! # IMAP copy
//!
//! The `imap copy` command, RFC 3501 `COPY`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use io_imap::{
    rfc3501::{
        copy::{ImapCopyUid, ImapMessageCopyOptions},
        select::ImapMailboxSelectOptions,
    },
    types::mailbox::Mailbox,
};
use log::warn;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag, TargetMailboxNameArg},
    outcome,
    utils::parse_sequence_set,
};

/// Copy messages to the given mailbox (COPY, RFC 3501).
#[derive(Debug, Parser)]
pub struct ImapMessageCopyCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,
    /// The messages to copy, as `1`, `1,2,3` or `1:*`.
    #[arg(name = "sequence_set", value_name = "SEQUENCE")]
    pub sequence_set: String,
    #[command(flatten)]
    pub mailbox_dest_name: TargetMailboxNameArg,
    /// Read the sequence set as message numbers rather than UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapMessageCopyCommand {
    /// Selects the source mailbox unless told not to, then copies.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        let sequence_set = parse_sequence_set(&self.sequence_set)?;
        let destination: Mailbox = self.mailbox_dest_name.inner.try_into()?;

        let result = (|| -> Result<_> {
            if !self.mailbox_no_select.inner {
                client.select(mailbox, ImapMailboxSelectOptions::default())?;
            }

            let copy_uid = client.copy(
                sequence_set,
                destination,
                ImapMessageCopyOptions { uid: !self.seq },
            )?;

            Ok(ImapMessageCopyOutput::new(copy_uid, "Copy completed"))
        })();

        outcome::print(printer, result)
    }
}

/// What `imap copy` and `imap move` understood of the server's answer:
/// the `COPYUID` response code (UIDPLUS), `null` and empty without it.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImapMessageCopyOutput {
    /// The UID validity of the destination mailbox.
    uid_validity: Option<u32>,
    /// The UIDs of the source messages, in the order of `destinationUids`.
    source_uids: Vec<u32>,
    /// The UIDs the messages got in the destination mailbox.
    destination_uids: Vec<u32>,
    /// What the text form says was done.
    #[serde(skip)]
    done: &'static str,
}

impl ImapMessageCopyOutput {
    /// The output of a `COPYUID`, if the server sent one it can list:
    /// one counting over [`COPYUID_MAX_PAIRS`] reads as none sent.
    pub(crate) fn new(copy_uid: Option<ImapCopyUid>, done: &'static str) -> Self {
        let mut output = Self {
            uid_validity: None,
            source_uids: Vec::new(),
            destination_uids: Vec::new(),
            done,
        };

        let Some(copy_uid) = copy_uid else {
            return output;
        };

        let count = copy_uid.count();
        if count > COPYUID_MAX_PAIRS {
            warn!("ignoring a COPYUID of {count} messages, too many to list");
            return output;
        }

        output.uid_validity = Some(copy_uid.uid_validity);
        for (source, destination) in copy_uid.pairs() {
            output.source_uids.push(source.get());
            output.destination_uids.push(destination.get());
        }

        output
    }
}

/// The most `COPYUID` pairs listed, its two sets being ranges a server
/// may announce in the billions.
const COPYUID_MAX_PAIRS: u64 = 1_000_000;

impl fmt::Display for ImapMessageCopyOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let join = |uids: &[u32]| {
            let uids: Vec<String> = uids.iter().map(u32::to_string).collect();
            uids.join(",")
        };

        // NOTE: a server sends no COPYUID without UIDPLUS, and a UIDPLUS
        // one sends none when nothing matched, so the plain form says
        // which UIDs moved only when it knows.
        if self.uid_validity.is_none() {
            return writeln!(f, "{}, no COPYUID returned", self.done);
        }

        let source = join(&self.source_uids);
        let destination = join(&self.destination_uids);
        writeln!(f, "{}: UIDs {source} to {destination}", self.done)
    }
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroU32, ops::RangeInclusive};

    use io_imap::rfc3501::copy::ImapCopyUid;

    use crate::imap::message::copy::ImapMessageCopyOutput;

    fn range(first: u32, last: u32) -> RangeInclusive<NonZeroU32> {
        NonZeroU32::new(first).unwrap()..=NonZeroU32::new(last).unwrap()
    }

    #[test]
    fn the_text_form_says_whether_a_copyuid_came_back() {
        let copy_uid = ImapCopyUid {
            uid_validity: 7,
            source: vec![range(1, 2)],
            destination: vec![range(10, 11)],
        };
        let copied = ImapMessageCopyOutput::new(Some(copy_uid), "Copy completed");
        let unknown = ImapMessageCopyOutput::new(None, "Copy completed");

        assert_eq!(copied.to_string(), "Copy completed: UIDs 1,2 to 10,11\n");
        assert_eq!(unknown.to_string(), "Copy completed, no COPYUID returned\n");
    }

    #[test]
    fn a_copyuid_pairs_each_source_with_its_destination() {
        let copy_uid = ImapCopyUid {
            uid_validity: 7,
            source: vec![range(5, 5), range(1, 2)],
            destination: vec![range(10, 12)],
        };
        let copied = ImapMessageCopyOutput::new(Some(copy_uid), "Copy completed");

        assert_eq!(copied.source_uids, [5, 1, 2]);
        assert_eq!(copied.destination_uids, [10, 11, 12]);
    }

    #[test]
    fn a_copyuid_of_billions_is_not_listed() {
        let copy_uid = ImapCopyUid {
            uid_validity: 7,
            source: vec![range(1, u32::MAX)],
            destination: vec![range(1, u32::MAX)],
        };
        let copied = ImapMessageCopyOutput::new(Some(copy_uid), "Copy completed");

        assert_eq!(copied.uid_validity, None);
        assert!(copied.source_uids.is_empty());
    }
}
