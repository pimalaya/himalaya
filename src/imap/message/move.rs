//! # IMAP move
//!
//! The `imap move` command, RFC 6851 `MOVE`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use io_imap::{
    rfc3501::select::ImapMailboxSelectOptions, rfc6851::r#move::ImapMessageMoveOptions,
    types::mailbox::Mailbox,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag, TargetMailboxNameArg},
};

/// Move messages to the given mailbox (MOVE, RFC 6851).
///
/// The server has to advertise the MOVE extension.
#[derive(Debug, Parser)]
pub struct ImapMessageMoveCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,
    /// The messages to move, as `1`, `1,2,3` or `1:*`.
    #[arg(name = "sequence_set", value_name = "SEQUENCE")]
    pub sequence_set: String,
    #[command(flatten)]
    pub mailbox_dest_name: TargetMailboxNameArg,
    /// Read the sequence set as message numbers rather than UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapMessageMoveCommand {
    /// Selects the source mailbox unless told not to, then moves.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            client.select(mailbox, ImapMailboxSelectOptions::default())?;
        }

        let sequence_set = self.sequence_set.as_str().try_into()?;
        let destination: Mailbox<'static> = self.mailbox_dest_name.inner.try_into()?;

        client.r#move(
            sequence_set,
            destination,
            ImapMessageMoveOptions { uid: !self.seq },
        )?;

        printer.out(Message::new("Message(s) successfully moved"))
    }
}
