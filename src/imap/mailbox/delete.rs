use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

/// Delete the given mailbox.
///
/// All emails from the given mailbox are definitely deleted. The mailbox is
/// also deleted after execution of the command.
#[derive(Debug, Parser)]
pub struct ImapMailboxDeleteCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;
        client.delete(mailbox)?;
        printer.out(Message::new("Mailbox successfully deleted"))
    }
}
