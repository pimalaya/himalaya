use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{account::MaildirAccount, arg::MaildirNameArg};

/// Create the given mailbox.
///
/// This command allows you to create a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct MaildirMailboxCreateCommand {
    #[command(flatten)]
    pub maildir_name: MaildirNameArg,
}

impl MaildirMailboxCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let path = account.backend.root.join(&self.maildir_name.inner);
        let client = account.new_maildir_client();
        client.create_maildir(path)?;
        printer.out(Message::new("Maildir successfully created"))
    }
}
