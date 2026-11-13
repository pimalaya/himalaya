//! # Maildir rename
//!
//! The `maildir rename` command, renaming a Maildir under the store
//! root.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use std::path::Path;

use crate::{
    maildir::{
        arg::{MaildirNameArg, RequiredMaildirPathFlag},
        client::{MaildirClient, validate_maildir_name},
    },
    shared::receipt::IdReceipt,
};

/// Rename a Maildir folder.
///
/// NAME is the full new path relative to the store root, so the folder
/// can move across the hierarchy, where `mailbox rename` keeps the parent.
/// The source is named explicitly, with no default, renaming being
/// destructive.
#[derive(Debug, Parser)]
pub struct MaildirMailboxRenameCommand {
    #[command(flatten)]
    pub maildir_path: RequiredMaildirPathFlag,
    #[command(flatten)]
    pub maildir_name: MaildirNameArg,
}

impl MaildirMailboxRenameCommand {
    /// Renames the Maildir under the store root.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        validate_maildir_name(&self.maildir_path.inner)?;
        validate_maildir_name(Path::new(&self.maildir_name.inner))?;

        // NOTE: io-maildir resolves both names relative to the store
        // root, so the bare names go through: pre-joining the root would
        // work under a second copy of it.
        let from = self.maildir_path.inner.to_string_lossy().into_owned();
        client.rename_maildir(from, self.maildir_name.inner.clone())?;

        printer.out(IdReceipt {
            id: self.maildir_name.inner,
            done: "Maildir successfully renamed",
        })
    }
}
