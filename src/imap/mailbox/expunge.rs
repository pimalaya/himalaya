use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameArg, MailboxNoSelectFlag},
};

/// Expunge the given mailbox.
///
/// All envelopes with the \Deleted flag will be definitely removed
/// from the given mailbox.
#[derive(Debug, Parser)]
pub struct ImapMailboxExpungeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,
}

impl ImapMailboxExpungeCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut client = account.new_imap_client()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            client.select(mailbox)?;
        }

        client.expunge()?;

        printer.out(Message::new("Mailbox successfully expunged"))
    }
}
