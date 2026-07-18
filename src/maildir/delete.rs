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

        // io-maildir resolves the name relative to the store root, so
        // pass the bare name — pre-joining the root here would make it
        // re-join and delete under `<root>/<root>` (or nothing).
        let path = self.maildir_path.inner.to_string_lossy().into_owned();
        client.delete_maildir(path)?;
        printer.out(Message::new("Maildir successfully deleted"))
    }
}
