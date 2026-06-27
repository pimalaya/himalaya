use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

/// Unsubscribe from the given mailbox (UNSUBSCRIBE, RFC 3501).
///
/// Removes the mailbox from the set of subscribed mailboxes.
#[derive(Debug, Parser)]
pub struct ImapMailboxUnsubscribeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxUnsubscribeCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;
        client.unsubscribe(mailbox)?;
        printer.out(Message::new("Mailbox successfully unsubscribed"))
    }
}
