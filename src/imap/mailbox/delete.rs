//! # IMAP delete
//!
//! The `imap delete` command, RFC 3501 `DELETE`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

/// Delete the given mailbox (DELETE, RFC 3501).
///
/// The mailbox and every message it holds go for good.
#[derive(Debug, Parser)]
pub struct ImapMailboxDeleteCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxDeleteCommand {
    /// Deletes the mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;
        client.delete(mailbox)?;
        printer.out(Message::new("Mailbox successfully deleted"))
    }
}
