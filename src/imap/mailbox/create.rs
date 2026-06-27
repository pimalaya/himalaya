use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

/// Create the given mailbox (CREATE, RFC 3501).
///
/// Creates a new mailbox with the given name.
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
