//! # IMAP unsubscribe
//!
//! The `imap unsubscribe` command, RFC 3501 `UNSUBSCRIBE`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::Printer;

use crate::imap::{
    client::ImapClient,
    mailbox::arg::MailboxNameArg,
    outcome::{self, ImapMailboxOutput},
};

/// Unsubscribe from the given mailbox (UNSUBSCRIBE, RFC 3501).
#[derive(Debug, Parser)]
pub struct ImapMailboxUnsubscribeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxUnsubscribeCommand {
    /// Unsubscribes from the mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let name = self.mailbox_name.inner;
        let mailbox = name.clone().try_into()?;

        let result = client
            .unsubscribe(mailbox)
            .map_err(Into::into)
            .map(|()| ImapMailboxOutput {
                mailbox: name,
                done: "IMAP mailbox successfully unsubscribed",
            });

        outcome::print(printer, result)
    }
}
