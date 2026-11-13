//! # Maildir create
//!
//! The `maildir create` command, laying out a new Maildir under the
//! store root.

use std::path::Path;

use anyhow::{Result, bail};
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    maildir::{
        arg::MaildirNameArg,
        client::{MaildirClient, validate_maildir_name},
    },
    shared::receipt::IdReceipt,
};

/// Create a Maildir folder.
///
/// Its `cur`, `new` and `tmp` subdirectories are laid out under the
/// store root. NAME is the path relative to the root, every level of it
/// one a listing shows: no `cur`, `new` or `tmp`, no leading dot. An
/// existing folder is refused.
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
        if Path::new(target.as_str()).exists() {
            bail!("Cannot create Maildir `{name}`: it already exists");
        }
        client.create_maildir(name.clone())?;

        printer.out(IdReceipt {
            id: name,
            done: "Maildir successfully created",
        })
    }
}
