//! # IMAP rename
//!
//! The `imap rename` command, RFC 3501 `RENAME`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::Printer;

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameArg, TargetMailboxNameArg},
    outcome::{self, ImapMailboxOutput},
};

/// Rename the given mailbox (RENAME, RFC 3501).
#[derive(Debug, Parser)]
pub struct ImapMailboxRenameCommand {
    #[command(flatten)]
    pub mailbox_source_name: MailboxNameArg,
    #[command(flatten)]
    pub mailbox_dest_name: TargetMailboxNameArg,
}

impl ImapMailboxRenameCommand {
    /// Renames the mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let name = self.mailbox_dest_name.inner;
        let from = self.mailbox_source_name.inner.try_into()?;
        let to = name.clone().try_into()?;

        let result = client
            .rename(from, to)
            .map_err(Into::into)
            .map(|()| ImapMailboxOutput {
                mailbox: name,
                done: "IMAP mailbox successfully renamed",
            });

        outcome::print(printer, result)
    }
}
