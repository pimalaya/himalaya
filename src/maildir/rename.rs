use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::{MaildirNameArg, MaildirPathFlag},
    client::MaildirClient,
};

/// Rename a Maildir folder.
///
/// Renames the folder directory from its current path to the new name.
#[derive(Debug, Parser)]
pub struct MaildirMailboxRenameCommand {
    #[command(flatten)]
    pub maildir_path: MaildirPathFlag,
    #[command(flatten)]
    pub maildir_name: MaildirNameArg,
}

impl MaildirMailboxRenameCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let path = client
            .root
            .join(&self.maildir_path.inner)
            .to_string_lossy()
            .into_owned();

        client.rename_maildir(path, self.maildir_name.inner)?;
        printer.out(Message::new("Maildir successfully renamed"))
    }
}
