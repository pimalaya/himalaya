use std::path::Path;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::MaildirNameArg,
    client::{MaildirClient, validate_maildir_name},
};

/// Create a Maildir folder.
///
/// Creates the new, cur and tmp subdirectories for a new folder under
/// the account root.
#[derive(Debug, Parser)]
pub struct MaildirMailboxCreateCommand {
    #[command(flatten)]
    pub maildir_name: MaildirNameArg,
}

impl MaildirMailboxCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        validate_maildir_name(Path::new(&self.maildir_name.inner))?;

        // io-maildir resolves the name relative to the store root, so
        // pass the bare name — pre-joining the root here would make it
        // re-join and land under `<root>/<root>`.
        client.create_maildir(self.maildir_name.inner)?;
        printer.out(Message::new("Maildir successfully created"))
    }
}
