use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

/// Create the given mailbox.
///
/// This command allows you to create a new mailbox using the given name.
#[derive(Debug, Parser)]
pub struct ImapMailboxCreateCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;
        client.create(mailbox)?;
        printer.out(Message::new("Mailbox successfully created"))
    }
}
