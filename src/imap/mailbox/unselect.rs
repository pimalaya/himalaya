//! # IMAP unselect
//!
//! The `imap unselect` command, RFC 3691 `UNSELECT`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::client::ImapClient;

/// Unselect the selected mailbox (UNSELECT, RFC 3691).
///
/// `CLOSE` without the expunge. A mailbox has to be selected, so this
/// wants a stateful IMAP session such as a Sirup proxy.
#[derive(Debug, Parser)]
pub struct ImapMailboxUnselectCommand;

impl ImapMailboxUnselectCommand {
    /// Unselects the selected mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        client.unselect()?;
        printer.out(Message::new("Mailbox successfully unselected"))
    }
}
