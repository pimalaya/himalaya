//! # IMAP create
//!
//! The `imap create` command, RFC 3501 `CREATE`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::Printer;

use crate::imap::{
    client::ImapClient,
    mailbox::arg::MailboxNameArg,
    outcome::{self, ImapMailboxOutput},
};

/// Create the given mailbox (CREATE, RFC 3501).
#[derive(Debug, Parser)]
pub struct ImapMailboxCreateCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxCreateCommand {
    /// Creates the mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let name = self.mailbox_name.inner;
        let mailbox = name.clone().try_into()?;

        let result = client
            .create(mailbox)
            .map_err(Into::into)
            .map(|()| ImapMailboxOutput {
                mailbox: name,
                done: "IMAP mailbox successfully created",
            });

        outcome::print(printer, result)
    }
}
