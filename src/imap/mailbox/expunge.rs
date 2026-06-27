use anyhow::Result;
use clap::Parser;
use io_imap::rfc3501::select::ImapMailboxSelectOptions;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameArg, MailboxNoSelectFlag},
};

/// Expunge the given mailbox (EXPUNGE, RFC 3501).
///
/// Permanently removes every message flagged \Deleted from the mailbox.
#[derive(Debug, Parser)]
pub struct ImapMailboxExpungeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,
}

impl ImapMailboxExpungeCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            client.select(mailbox, ImapMailboxSelectOptions::default())?;
        }

        client.expunge()?;

        printer.out(Message::new("Mailbox successfully expunged"))
    }
}
