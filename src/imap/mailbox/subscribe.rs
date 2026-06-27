use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

/// Subscribe to the given mailbox (SUBSCRIBE, RFC 3501).
///
/// Adds the mailbox to the set of subscribed mailboxes.
#[derive(Debug, Parser)]
pub struct ImapMailboxSubscribeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxSubscribeCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;
        client.subscribe(mailbox)?;
        printer.out(Message::new("Mailbox successfully subscribed"))
    }
}
