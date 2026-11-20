//! # Maildir create
//!
//! The `maildir create` command, laying out a new Maildir under the
//! store root.

use std::path::Path;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    maildir::{
        arg::MaildirNameArg,
        client::{MaildirClient, refuse_existing, validate_maildir_name},
    },
    shared::receipt::IdReceipt,
};

/// Create a Maildir folder.
///
/// Its `cur`, `new` and `tmp` subdirectories are laid out under the
/// store root. NAME is the path relative to the root, every level of it
/// one a listing shows: no `cur`, `new` or `tmp`, no leading dot. An
/// existing folder is refused, coded `mailbox-duplicate`.
#[derive(Debug, Parser)]
pub struct MaildirMailboxCreateCommand {
    #[command(flatten)]
    pub maildir_name: MaildirNameArg,
}

impl MaildirMailboxCreateCommand {
    /// Creates the Maildir under the store root.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let name = Path::new(&self.maildir_name.inner);
        validate_maildir_name(name)?;

        let name = client.logical_name(name)?;
        for level in name.split('/') {
            client.validate_level(level)?;
        }

        // NOTE: io-maildir resolves a name relative to the store root, so
        // the bare name goes through: pre-joining the root would land the
        // Maildir under a second copy of it.
        let target = client.store.resolve(&name.as_str().into());
        refuse_existing(Path::new(target.as_str()), &name)?;
        client.create_maildir(name)?;

        printer.out(IdReceipt {
            id: target.as_str().to_owned(),
            done: "Maildir successfully created",
        })
    }
}
