use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::RequiredMaildirPathFlag,
    client::{MaildirClient, validate_maildir_name},
};

/// Delete a Maildir folder.
///
/// Removes the folder directory and every message it contains. The
/// target must be given explicitly (no default), since deletion is
/// destructive.
#[derive(Debug, Parser)]
pub struct MaildirMailboxDeleteCommand {
    #[command(flatten)]
    pub maildir_path: RequiredMaildirPathFlag,
}

impl MaildirMailboxDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        validate_maildir_name(&self.maildir_path.inner)?;

        let path = client
            .root
            .join(&self.maildir_path.inner)
            .to_string_lossy()
            .into_owned();

        client.delete_maildir(path)?;
        printer.out(Message::new("Maildir successfully deleted"))
    }
}
