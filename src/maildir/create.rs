use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{arg::MaildirNameArg, client::MaildirClient};

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
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let path = client
            .root
            .join(&self.maildir_name.inner)
            .to_string_lossy()
            .into_owned();

        client.create_maildir(path)?;
        printer.out(Message::new("Maildir successfully created"))
    }
}
