use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use std::path::Path;

use crate::maildir::{
    arg::{MaildirNameArg, RequiredMaildirPathFlag},
    client::{MaildirClient, validate_maildir_name},
};

/// Rename a Maildir folder.
///
/// Renames the folder directory from its current path to the new name.
/// The source must be given explicitly (no default), since renaming is
/// destructive.
#[derive(Debug, Parser)]
pub struct MaildirMailboxRenameCommand {
    #[command(flatten)]
    pub maildir_path: RequiredMaildirPathFlag,
    #[command(flatten)]
    pub maildir_name: MaildirNameArg,
}

impl MaildirMailboxRenameCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        validate_maildir_name(&self.maildir_path.inner)?;
        validate_maildir_name(Path::new(&self.maildir_name.inner))?;

        let path = client
            .root
            .join(&self.maildir_path.inner)
            .to_string_lossy()
            .into_owned();

        client.rename_maildir(path, self.maildir_name.inner)?;
        printer.out(Message::new("Maildir successfully renamed"))
    }
}
