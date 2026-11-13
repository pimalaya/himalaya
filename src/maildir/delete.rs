//! # Maildir delete
//!
//! The `maildir delete` command, removing a Maildir and its messages.

use anyhow::{Result, bail};
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    maildir::{arg::RequiredMaildirPathFlag, client::MaildirClient},
    shared::receipt::IdReceipt,
};

/// Delete a Maildir folder.
///
/// The directory goes, with every message in it and every folder below
/// it, nested Maildirs included, where `mailbox delete` refuses a
/// Maildir holding others. The root cannot be deleted. The target is
/// named explicitly, with no default, deletion being destructive.
#[derive(Debug, Parser)]
pub struct MaildirMailboxDeleteCommand {
    #[command(flatten)]
    pub maildir_path: RequiredMaildirPathFlag,
}

impl MaildirMailboxDeleteCommand {
    /// Deletes the Maildir and every message in it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let maildir = client.resolve_maildir(&self.maildir_path.inner)?;
        if client.is_root(&maildir) {
            let path = self.maildir_path.inner.display();
            bail!("Cannot delete Maildir `{path}`: it is the store root");
        }

        // NOTE: io-maildir resolves a name relative to the store root, so
        // the root-relative name goes through: the absolute path would
        // land under a second copy of the root.
        let name = client.relative_name(&maildir);
        client.delete_maildir(name.clone())?;

        printer.out(IdReceipt {
            id: name,
            done: "Maildir successfully deleted",
        })
    }
}
