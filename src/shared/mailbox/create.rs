//! # Mailbox create
//!
//! The `mailbox create` command, creating one mailbox at the top of the
//! account or under another.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{printer::Printer, table::sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, shared::client::EmailClient};

/// Create a mailbox.
///
/// The name is one level of the hierarchy: the mailbox is created at the
/// top of the account, or under the one `--parent` names. A backend
/// naming the hierarchy by path joins the two with its own delimiter.
#[derive(Debug, Parser)]
pub struct MailboxCreateCommand {
    /// The new mailbox's name, one level of the hierarchy.
    #[arg(value_name = "NAME")]
    pub name: String,
    /// The mailbox to create it under, by name, alias, role or id.
    #[arg(long, value_name = "MAILBOX")]
    pub parent: Option<String>,
}

impl MailboxCreateCommand {
    /// Creates the mailbox and reports its id.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let parent = self
            .parent
            .as_deref()
            .map(|parent| account.resolve_mailbox(parent).to_string());

        let id = client.create_mailbox(&self.name, parent.as_deref())?;

        printer.out(MailboxCreateOutput {
            id,
            name: self.name,
            queued: client.queues_writes(),
        })
    }
}

/// The `mailbox create` output.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MailboxCreateOutput {
    /// The new mailbox's id, `null` when the backend queued the creation
    /// for its sync engine, the id arriving with the sync.
    pub id: Option<String>,
    /// The name it was created with.
    pub name: String,
    /// Whether a sync engine applies the write later rather than now.
    pub queued: bool,
}

impl fmt::Display for MailboxCreateOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = sanitize(&self.name);
        match &self.id {
            _ if self.queued => writeln!(f, "Mailbox `{name}` queued for creation"),
            Some(id) => writeln!(
                f,
                "Successfully created mailbox `{name}` as `{}`",
                sanitize(id)
            ),
            None => writeln!(f, "Successfully created mailbox `{name}`"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::mailbox::create::MailboxCreateOutput;

    #[test]
    fn a_queued_creation_has_a_null_id_and_says_queued() {
        let output = MailboxCreateOutput {
            id: None,
            name: "Archive".into(),
            queued: true,
        };

        assert_eq!(
            output.to_string(),
            "Mailbox `Archive` queued for creation\n"
        );
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            r#"{"id":null,"name":"Archive","queued":true}"#
        );
    }
}
