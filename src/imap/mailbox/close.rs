use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::account::ImapAccount;

/// Close the current, selected mailbox.
///
/// This command also expunges the current mailbox and returns to the
/// authenticated state.
///
/// NOTE: Since a selected mailbox is required, this command only
/// works for stateful IMAP sessions. See:
///
/// https://github.com/pimalaya/sirup
#[derive(Debug, Parser)]
pub struct ImapMailboxCloseCommand;

impl ImapMailboxCloseCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut client = account.new_imap_client()?;
        client.close()?;
        printer.out(Message::new("Mailbox successfully closed"))
    }
}
