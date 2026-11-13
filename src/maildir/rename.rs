//! # Maildir rename
//!
//! The `maildir rename` command, renaming a Maildir under the store
//! root.

use std::path::Path;

use anyhow::{Result, bail};
use clap::Parser;
use pimalaya_cli::printer::Printer;

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
/// Its parent must exist, and the folder must not. The root cannot be
/// renamed. The source is named explicitly, with no default, renaming
/// being destructive.
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
        let maildir = client.resolve_maildir(&self.maildir_path.inner)?;
        if client.is_root(&maildir) {
            let path = self.maildir_path.inner.display();
            bail!("Cannot rename Maildir `{path}`: it is the store root");
        }

        let to = Path::new(&self.maildir_name.inner);
        validate_maildir_name(to)?;
        let to = client.logical_name(to)?;
        for level in to.split('/') {
            client.validate_level(level)?;
        }

        let target = client.store.resolve(&to.as_str().into());
        if Path::new(target.as_str()).exists() {
            bail!("Cannot rename Maildir to `{to}`: it already exists");
        }
        // NOTE: Maildir++ keeps every folder flat at the root, so a
        // parent exists there only by name.
        if let Some((parent, _)) = to.rsplit_once('/')
            && !client.store.maildirpp
            && !client.root.join(parent).is_dir()
        {
            bail!("Cannot rename Maildir to `{to}`: its parent `{parent}` does not exist");
        }

        // NOTE: io-maildir resolves both names relative to the store
        // root, so the root-relative names go through: absolute paths
        // would work under a second copy of the root.
        let from = client.relative_name(&maildir);
        client.rename_maildir(from, to.clone())?;

        printer.out(IdReceipt {
            id: to,
            done: "Maildir successfully renamed",
        })
    }
}
