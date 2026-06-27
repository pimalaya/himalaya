use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{arg::MaildirPathFlag, client::MaildirClient};

/// Delete a Maildir folder.
///
/// Removes the folder directory and every message it contains.
#[derive(Debug, Parser)]
pub struct MaildirMailboxDeleteCommand {
    #[command(flatten)]
    pub maildir_path: MaildirPathFlag,
}

impl MaildirMailboxDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let path = client
            .root
            .join(&self.maildir_path.inner)
            .to_string_lossy()
            .into_owned();

        client.delete_maildir(path)?;
        printer.out(Message::new("Maildir successfully deleted"))
    }
}
