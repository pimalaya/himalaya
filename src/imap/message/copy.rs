//! # IMAP copy
//!
//! The `imap copy` command, RFC 3501 `COPY`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use io_imap::{
    rfc3501::{copy::ImapMessageCopyOptions, select::ImapMailboxSelectOptions},
    types::mailbox::Mailbox,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag, TargetMailboxNameArg},
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

        if !self.mailbox_no_select.inner {
            client.select(mailbox, ImapMailboxSelectOptions::default())?;
        }

        let sequence_set = self.sequence_set.as_str().try_into()?;
        let destination: Mailbox = self.mailbox_dest_name.inner.try_into()?;

        client.copy(
            sequence_set,
            destination,
            ImapMessageCopyOptions { uid: !self.seq },
        )?;

        printer.out(Message::new("Message(s) successfully copied"))
    }
}
