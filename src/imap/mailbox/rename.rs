use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameArg, TargetMailboxNameArg},
};

/// Rename the given mailbox (RENAME, RFC 3501).
///
/// Renames an existing mailbox to a new name.
#[derive(Debug, Parser)]
pub struct ImapMailboxRenameCommand {
    #[command(flatten)]
    pub mailbox_source_name: MailboxNameArg,
    #[command(flatten)]
    pub mailbox_dest_name: TargetMailboxNameArg,
}

impl ImapMailboxRenameCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let from = self.mailbox_source_name.inner.try_into()?;
        let to = self.mailbox_dest_name.inner.try_into()?;
        client.rename(from, to)?;
        printer.out(Message::new("Mailbox successfully renamed"))
    }
}
