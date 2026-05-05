use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

/// Subscribe to the given mailbox.
///
/// This command subscribes to a mailbox, making it appear in the
/// list of subscribed mailboxes.
#[derive(Debug, Parser)]
pub struct ImapMailboxSubscribeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxSubscribeCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut client = account.new_imap_client()?;
        let mailbox = self.mailbox_name.inner.try_into()?;
        client.subscribe(mailbox)?;
        printer.out(Message::new("Mailbox successfully subscribed"))
    }
}
