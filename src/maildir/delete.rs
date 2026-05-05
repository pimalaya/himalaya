use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{account::MaildirAccount, arg::MaildirPathFlag};

/// Delete the given mailbox.
///
/// This command allows you to delete a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct MaildirMailboxDeleteCommand {
    #[command(flatten)]
    pub maildir_path: MaildirPathFlag,
}

impl MaildirMailboxDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let path = account.backend.root.join(&self.maildir_path.inner);
        let client = account.new_maildir_client();
        client.delete_maildir(path)?;
        printer.out(Message::new("Maildir successfully deleted"))
    }
}
