//! # Mailbox rename
//!
//! The `mailbox rename` command, giving one mailbox a new name under the
//! same parent.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, shared::client::EmailClient};

/// Rename a mailbox, keeping its parent.
///
/// The new name is one level of the hierarchy: it replaces the last level
/// of the mailbox's name, the mailbox staying where it is with its
/// messages. On a backend naming mailboxes by path, the id changes with
/// the name.
#[derive(Debug, Parser)]
pub struct MailboxRenameCommand {
    /// The mailbox to rename, by name, alias, role or id.
    #[arg(value_name = "MAILBOX")]
    pub mailbox: String,
    /// Its new name, one level of the hierarchy.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl MailboxRenameCommand {
    /// Renames the mailbox and reports the id it has afterwards.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = account.resolve_mailbox(&self.mailbox).to_string();
        let id = client.rename_mailbox(&mailbox, &self.name)?;

        printer.out(MailboxRenameOutput {
            id,
            name: self.name,
            mailbox,
            queued: client.queues_writes(),
        })
    }
}

/// The `mailbox rename` output.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MailboxRenameOutput {
    /// The mailbox's id once renamed, which a backend naming mailboxes by
    /// path changes, `null` when the backend queued the rename.
    pub id: Option<String>,
    /// Its new name.
    pub name: String,
    /// The mailbox as it was given, for the confirmation.
    #[serde(skip)]
    pub mailbox: String,
    /// Whether a sync engine applies the write later rather than now.
    pub queued: bool,
}

impl fmt::Display for MailboxRenameOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (mailbox, name) = (sanitize::text(&self.mailbox), sanitize::text(&self.name));
        if self.queued {
            return writeln!(f, "Rename of mailbox `{mailbox}` to `{name}` queued");
        }
        writeln!(f, "Successfully renamed mailbox `{mailbox}` to `{name}`")
    }
}
