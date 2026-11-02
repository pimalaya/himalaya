//! # pimdir mailbox rename
//!
//! The `pimdir mailbox rename` command, queueing a `collection-rename`
//! intent for the store's owner.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{printer::Printer, table::sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, pimdir::client::PimdirClient};

/// Queue the renaming of a mailbox, printing its queue row.
///
/// The sync engine's source that can rename this mailbox renames it on its
/// server, on its next run, keeping its parent and its messages; the new
/// name shows once that sync brings it down. `pimdir queue show` follows
/// the row, and says why when the server refuses, a sibling already holding
/// the name or a name holding the server's hierarchy separator for
/// instance.
#[derive(Debug, Parser)]
pub struct PimdirMailboxRenameCommand {
    /// The mailbox to rename, by id, role or alias.
    #[arg(value_name = "MAILBOX")]
    pub mailbox: String,
    /// Its new name on the server, one level of the hierarchy.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl PimdirMailboxRenameCommand {
    /// Queues the intent and prints its row.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let mailbox = account.resolve_mailbox(&self.mailbox).to_owned();
        let renamed = client.rename_mailbox(&mailbox, &self.name)?;

        printer.out(PimdirMailboxRenameOutput {
            queue_id: renamed.queue_id,
            mailbox,
            name: self.name,
            source: renamed.source,
        })
    }
}

/// The `pimdir mailbox rename` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailboxRenameOutput {
    /// The `collection-rename` row.
    pub queue_id: i64,
    /// The mailbox renamed, as given once its alias is resolved.
    pub mailbox: String,
    /// Its new name.
    pub name: String,
    /// The source renaming it.
    pub source: String,
}

impl fmt::Display for PimdirMailboxRenameOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Mailbox {} queued for renaming to {} by {} as row {}",
            sanitize(&self.mailbox),
            sanitize(&self.name),
            sanitize(&self.source),
            self.queue_id
        )
    }
}
