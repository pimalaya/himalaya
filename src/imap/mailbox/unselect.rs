use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::account::ImapAccount;

/// Unselect a current, selected mailbox.
///
/// Unlike CLOSE, UNSELECT does not expunge deleted messages.
///
/// NOTE: Since a selected mailbox is required, this command only
/// works for stateful IMAP sessions. See:
///
/// https://github.com/pimalaya/sirup
#[derive(Debug, Parser)]
pub struct ImapMailboxUnselectCommand;

impl ImapMailboxUnselectCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut client = account.new_imap_client()?;
        client.unselect()?;
        printer.out(Message::new("Mailbox successfully unselected"))
    }
}
