//! # IMAP unsubscribe
//!
//! The `imap unsubscribe` command, RFC 3501 `UNSUBSCRIBE`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

/// Unsubscribe from the given mailbox (UNSUBSCRIBE, RFC 3501).
#[derive(Debug, Parser)]
pub struct ImapMailboxUnsubscribeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxUnsubscribeCommand {
    /// Unsubscribes from the mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;
        client.unsubscribe(mailbox)?;
        printer.out(Message::new("Mailbox successfully unsubscribed"))
    }
}
