//! # IMAP close
//!
//! The `imap close` command, RFC 3501 `CLOSE`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::client::ImapClient;

/// Close the selected mailbox (CLOSE, RFC 3501).
///
/// Expunges the messages flagged `\Deleted` and returns to the
/// authenticated state. A mailbox has to be selected, so this wants a
/// stateful IMAP session such as a Sirup proxy.
#[derive(Debug, Parser)]
pub struct ImapMailboxCloseCommand;

impl ImapMailboxCloseCommand {
    /// Closes the selected mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        client.close()?;
        printer.out(Message::new("Mailbox successfully closed"))
    }
}
