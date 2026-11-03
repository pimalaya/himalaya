//! # IMAP delete
//!
//! The `imap delete` command, RFC 3501 `DELETE`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::Printer;

use crate::imap::{
    client::ImapClient,
    mailbox::arg::MailboxNameArg,
    outcome::{self, ImapMailboxOutput},
};

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
        let name = self.mailbox_name.inner;
        let mailbox = name.clone().try_into()?;

        let result = client
            .delete(mailbox)
            .map_err(Into::into)
            .map(|()| ImapMailboxOutput {
                mailbox: name,
                done: "IMAP mailbox successfully deleted",
            });

        outcome::print(printer, result)
    }
}
