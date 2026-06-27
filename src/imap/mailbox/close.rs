use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::client::ImapClient;

/// Close the selected mailbox (CLOSE, RFC 3501).
///
/// Expunges \Deleted messages and returns to the authenticated state.
///
/// NOTE: requires a selected mailbox, so this only works within a
/// stateful IMAP session. See https://github.com/pimalaya/sirup
#[derive(Debug, Parser)]
pub struct ImapMailboxCloseCommand;

impl ImapMailboxCloseCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        client.close()?;
        printer.out(Message::new("Mailbox successfully closed"))
    }
}
