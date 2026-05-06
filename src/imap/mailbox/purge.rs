use anyhow::Result;
use clap::Parser;
use io_imap::types::flag::{Flag, StoreType};
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameArg, MailboxNoSelectFlag},
};

/// Shortcut for marking as deleted all envelopes then expunging the
/// given mailbox.
///
/// All emails from the given mailbox are definitely deleted. The
/// purged mailbox will remain empty after execution of the command.
#[derive(Debug, Parser)]
pub struct ImapMailboxPurgeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,
}

impl ImapMailboxPurgeCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            client.select(mailbox)?;
        }

        client.store(
            "1:*".try_into()?,
            StoreType::Add,
            vec![Flag::Deleted],
            false,
        )?;
        client.expunge()?;

        printer.out(Message::new("Mailbox successfully purged"))
    }
}
