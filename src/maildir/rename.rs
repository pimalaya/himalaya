use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::{MaildirNameArg, MaildirPathFlag},
    client::MaildirClient,
};

/// Rename the given mailbox.
///
/// This command allows you to rename a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct MaildirMailboxRenameCommand {
    #[command(flatten)]
    pub maildir_path: MaildirPathFlag,
    #[command(flatten)]
    pub maildir_name: MaildirNameArg,
}

impl MaildirMailboxRenameCommand {
    pub fn execute(self, printer: &mut impl Printer, client: MaildirClient) -> Result<()> {
        let path = client.root.join(&self.maildir_path.inner);

        client.rename_maildir(path, self.maildir_name.inner)?;
        printer.out(Message::new("Maildir successfully renamed"))
    }
}
