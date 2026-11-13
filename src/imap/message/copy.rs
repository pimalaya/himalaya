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
    /// The output of a `COPYUID`, if the server sent one.
    pub(crate) fn new(copy_uid: ImapCopyUid, done: &'static str) -> Self {
        let (uid_validity, source_uids, destination_uids) = match copy_uid {
            Some((validity, source, destination)) => (Some(validity), source, destination),
            None => (None, Vec::new(), Vec::new()),
        };

        Self {
            uid_validity,
            source_uids,
            destination_uids,
            done,
        }
    }
}

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
    use crate::imap::message::copy::ImapMessageCopyOutput;

    #[test]
    fn the_text_form_says_whether_a_copyuid_came_back() {
        let copied =
            ImapMessageCopyOutput::new(Some((7, vec![1, 2], vec![10, 11])), "Copy completed");
        let unknown = ImapMessageCopyOutput::new(None, "Copy completed");

        assert_eq!(copied.to_string(), "Copy completed: UIDs 1,2 to 10,11\n");
        assert_eq!(unknown.to_string(), "Copy completed, no COPYUID returned\n");
    }
}
