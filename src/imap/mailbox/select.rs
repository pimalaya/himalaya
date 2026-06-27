use anyhow::Result;
use clap::Parser;
use io_imap::rfc3501::select::ImapMailboxSelectOptions;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg};

/// Select the given mailbox (SELECT, RFC 3501).
///
/// Opens the mailbox for read-write access and returns its status
/// (flags, message count, UID validity, ...).
///
/// NOTE: a selected mailbox only persists within a stateful IMAP
/// session. See https://github.com/pimalaya/sirup
#[derive(Debug, Parser)]
pub struct ImapMailboxSelectCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxSelectCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;
        client.select(mailbox, ImapMailboxSelectOptions::default())?;
        printer.out(Message::new("Mailbox successfully selected"))
    }
}
