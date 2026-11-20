//! # pimdir mailbox delete
//!
//! The `pimdir mailbox delete` command, queueing a `collection-delete`
//! intent for the store's owner.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, pimdir::client::PimdirClient};

/// Queue the deletion of a mailbox and its messages, printing its queue
/// row.
///
/// The sync engine's source that can delete this mailbox deletes it on its
/// server, on its next run, with every message it holds there; the mailbox
/// leaves the store once that sync brings the deletion down. A mailbox
/// holding other mailboxes is never deleted with them: the row parks naming
/// them, `pimdir queue show` says so, and the others are deleted first.
#[derive(Debug, Parser)]
pub struct PimdirMailboxDeleteCommand {
    /// The mailbox to delete, by id, role or alias.
    #[arg(value_name = "MAILBOX")]
    pub mailbox: String,
}

impl PimdirMailboxDeleteCommand {
    /// Queues the intent and prints its row.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let mailbox = account.resolve_mailbox(&self.mailbox).to_owned();
        let deleted = client.delete_mailbox(&mailbox)?;

        printer.out(PimdirMailboxDeleteOutput {
            id: deleted.mailbox.unwrap_or_else(|| mailbox.clone()),
            row: deleted.queue_id.id,
            handle: deleted.queue_id.to_string(),
            mailbox,
            source: deleted.source,
        })
    }
}

/// The `pimdir mailbox delete` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailboxDeleteOutput {
    /// The id of the mailbox deleted.
    pub id: String,
    /// The `collection-delete` row.
    pub row: i64,
    /// The row's handle, its id under the store's identity (pimdir STORAGE
    /// §4.5), which `pimdir queue show` and `cancel` take and check.
    pub handle: String,
    /// The mailbox deleted, as given once its alias is resolved.
    pub mailbox: String,
    /// The source deleting it.
    pub source: String,
}

impl fmt::Display for PimdirMailboxDeleteOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Mailbox {} queued for deletion by {} as row {} (handle {})",
            sanitize::text(&self.mailbox),
            sanitize::text(&self.source),
            self.row,
            self.handle,
        )
    }
}
