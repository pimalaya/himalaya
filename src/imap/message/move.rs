use anyhow::Result;
use clap::Parser;
use io_imap::{
    rfc3501::select::ImapMailboxSelectOptions, rfc6851::r#move::ImapMessageMoveOptions,
    types::mailbox::Mailbox,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag, TargetMailboxNameArg},
};

/// Move IMAP message(s) to the given mailbox (MOVE, RFC 6851).
///
/// Moves the messages in the sequence set from the source mailbox to
/// the destination mailbox. Requires the MOVE extension.
#[derive(Debug, Parser)]
pub struct ImapMessageMoveCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

    /// The sequence set of messages (e.g., "1", "1,2,3", "1:*").
    #[arg(name = "sequence_set", value_name = "SEQUENCE")]
    pub sequence_set: String,
    #[command(flatten)]
    pub mailbox_dest_name: TargetMailboxNameArg,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapMessageMoveCommand {
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
