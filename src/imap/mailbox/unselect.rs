use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::client::ImapClient;

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
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        client.unselect()?;
        printer.out(Message::new("Mailbox successfully unselected"))
    }
}
