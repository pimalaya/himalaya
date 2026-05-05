use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameArg, TargetMailboxNameArg},
};

/// Rename the given mailbox.
///
/// This command renames an existing mailbox to a new name.
#[derive(Debug, Parser)]
pub struct ImapMailboxRenameCommand {
    #[command(flatten)]
    pub mailbox_source_name: MailboxNameArg,
    #[command(flatten)]
    pub mailbox_dest_name: TargetMailboxNameArg,
}

impl ImapMailboxRenameCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut client = account.new_imap_client()?;
        let from = self.mailbox_source_name.inner.try_into()?;
        let to = self.mailbox_dest_name.inner.try_into()?;
        client.rename(from, to)?;
        printer.out(Message::new("Mailbox successfully renamed"))
    }
}
