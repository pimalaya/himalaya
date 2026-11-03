//! # IMAP subscribe
//!
//! The `imap subscribe` command, RFC 3501 `SUBSCRIBE`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::Printer;

use crate::imap::{
    client::ImapClient,
    mailbox::arg::MailboxNameArg,
    outcome::{self, ImapMailboxOutput},
};

/// Subscribe to the given mailbox (SUBSCRIBE, RFC 3501).
#[derive(Debug, Parser)]
pub struct ImapMailboxSubscribeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxSubscribeCommand {
    /// Subscribes to the mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let name = self.mailbox_name.inner;
        let mailbox = name.clone().try_into()?;

        let result = client
            .subscribe(mailbox)
            .map_err(Into::into)
            .map(|()| ImapMailboxOutput {
                mailbox: name,
                done: "IMAP mailbox successfully subscribed",
            });

        outcome::print(printer, result)
    }
}
