use anyhow::Result;
use clap::Parser;
use io_imap::rfc3501::select::ImapMailboxSelectOptions;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

/// Select the given mailbox.
///
/// This command permanently removes all messages with the \Deleted
/// flag and returns to the authenticated state.
///
/// NOTE: This command only works for stateful IMAP sessions. See:
///
/// https://github.com/pimalaya/sirup
#[derive(Debug, Parser)]
pub struct ImapMailboxSelectCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxSelectCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;
        client.select(mailbox, ImapMailboxSelectOptions::default())?;
        printer.out(Message::new("Mailbox successfully selected"))
    }
}
