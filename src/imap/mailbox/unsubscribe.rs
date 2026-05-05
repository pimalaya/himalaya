use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

/// Unsubscribe from the given mailbox.
///
/// This command unsubscribes from a mailbox, removing it from the
/// list of subscribed mailboxes.
#[derive(Debug, Parser)]
pub struct ImapMailboxUnsubscribeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxUnsubscribeCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut client = account.new_imap_client()?;
        let mailbox = self.mailbox_name.inner.try_into()?;
        client.unsubscribe(mailbox)?;
        printer.out(Message::new("Mailbox successfully unsubscribed"))
    }
}
