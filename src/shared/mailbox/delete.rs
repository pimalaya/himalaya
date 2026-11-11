//! # Mailbox delete
//!
//! The `mailbox delete` command, deleting one mailbox and the messages it
//! holds.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, shared::client::EmailClient};

/// Delete a mailbox and the messages it holds.
///
/// The mailbox is named explicitly, with no default, deletion being
/// destructive. What happens to a message the mailbox shares with another,
/// and to the mailboxes it holds, depends on the backend, as the footer
/// below says.
#[derive(Debug, Parser)]
pub struct MailboxDeleteCommand {
    /// The mailbox to delete, by name, alias, role or id.
    #[arg(value_name = "MAILBOX")]
    pub mailbox: String,
}

impl MailboxDeleteCommand {
    /// Deletes the mailbox and reports its id.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = account.resolve_mailbox(&self.mailbox).to_string();
        let id = client.delete_mailbox(&mailbox)?;

        printer.out(MailboxDeleteOutput {
            id,
            queued: client.queues_writes(),
        })
    }
}

/// The `mailbox delete` output.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MailboxDeleteOutput {
    /// The id of the mailbox deleted.
    pub id: String,
    /// Whether a sync engine applies the write later rather than now.
    pub queued: bool,
}

impl fmt::Display for MailboxDeleteOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = sanitize::text(&self.id);
        if self.queued {
            return writeln!(f, "Deletion of mailbox `{id}` queued");
        }
        writeln!(f, "Successfully deleted mailbox `{id}`")
    }
}
