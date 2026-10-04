//! # pimdir mailbox create
//!
//! The `pimdir mailbox create` command, queueing a `collection-create`
//! intent for the store's owner.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{printer::Printer, table::sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, pimdir::client::PimdirClient};

/// Queue the creation of a mailbox, printing its queue row.
///
/// The sync engine's source that can create mailboxes for this account
/// creates it on its server, on its next run; the mailbox shows once that
/// sync brings it down. `pimdir queue show` follows the row, and says why
/// when the server refuses, a name it already holds for instance.
#[derive(Debug, Parser)]
pub struct PimdirMailboxCreateCommand {
    /// The new mailbox's name on the server.
    #[arg(value_name = "NAME")]
    pub name: String,
    /// The mailbox to create it in, by name or alias; omitted, at the top.
    #[arg(long, value_name = "MAILBOX")]
    pub parent: Option<String>,
}

impl PimdirMailboxCreateCommand {
    /// Queues the intent and prints its row.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let parent = self
            .parent
            .as_deref()
            .map(|parent| account.resolve_mailbox(parent).to_owned());
        let created = client.create_mailbox(&self.name, parent.as_deref())?;

        printer.out(PimdirMailboxCreated {
            queue_id: created.queue_id,
            name: self.name,
            parent,
            source: created.source,
        })
    }
}

/// The `pimdir mailbox create` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailboxCreated {
    /// The `collection-create` row.
    pub queue_id: i64,
    /// The new mailbox's name.
    pub name: String,
    /// The mailbox it is created in, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The source creating it.
    pub source: String,
}

impl fmt::Display for PimdirMailboxCreated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Mailbox {} queued for creation by {} as row {}",
            sanitize(&self.name),
            sanitize(&self.source),
            self.queue_id
        )
    }
}
