use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::client::ImapClient;

/// Unselect the selected mailbox (UNSELECT, RFC 3691).
///
/// Like CLOSE but does not expunge \Deleted messages.
///
/// NOTE: requires a selected mailbox, so this only works within a
/// stateful IMAP session. See https://github.com/pimalaya/sirup
#[derive(Debug, Parser)]
pub struct ImapMailboxUnselectCommand;

impl ImapMailboxUnselectCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        client.unselect()?;
        printer.out(Message::new("Mailbox successfully unselected"))
    }
}
