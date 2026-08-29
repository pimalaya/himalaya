//! # IMAP select
//!
//! The `imap select` command, RFC 3501 `SELECT`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use io_imap::rfc3501::select::ImapMailboxSelectOptions;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

/// Select the given mailbox (SELECT, RFC 3501).
///
/// Opens it for read-write access and returns its status. The selection
/// only outlives the command over a stateful IMAP session such as a Sirup
/// proxy.
#[derive(Debug, Parser)]
pub struct ImapMailboxSelectCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxSelectCommand {
    /// Selects the mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;
        client.select(mailbox, ImapMailboxSelectOptions::default())?;
        printer.out(Message::new("Mailbox successfully selected"))
    }
}
