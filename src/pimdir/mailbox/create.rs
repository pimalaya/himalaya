//! # pimdir mailbox create
//!
//! The `pimdir mailbox create` command, queueing a `collection-create`
//! intent for the store's owner.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{printer::Printer, sanitize};
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

        printer.out(PimdirMailboxCreateOutput {
            row: created.queue_id.id,
            handle: created.queue_id.to_string(),
            name: self.name,
            parent,
            source: created.source,
        })
    }
}

/// The `pimdir mailbox create` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailboxCreateOutput {
    /// The `collection-create` row.
    pub row: i64,
    /// The row's handle, its id under the store's identity (pimdir STORAGE
    /// §4.5), which `pimdir queue show` and `cancel` take and check.
    pub handle: String,
    /// The new mailbox's name.
    pub name: String,
    /// The mailbox it is created in, `null` at the top.
    pub parent: Option<String>,
    /// The source creating it.
    pub source: String,
}

impl fmt::Display for PimdirMailboxCreateOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Mailbox {} queued for creation by {} as row {} (handle {})",
            sanitize::text(&self.name),
            sanitize::text(&self.source),
            self.row,
            self.handle,
        )
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::clap::commands::json_schema::output_schema;
    use serde_json::{Value, json};

    use super::PimdirMailboxCreateOutput;

    /// A mailbox created at the top prints its `parent` as `null`, a key
    /// the schema requires, rather than leaving it out.
    #[test]
    fn a_mailbox_at_the_top_prints_a_null_parent() {
        let output = PimdirMailboxCreateOutput {
            row: 89,
            handle: "e2646df031cd876334410c4df14ebd86.1.89".into(),
            name: String::from("Projets"),
            parent: None,
            source: String::from("google/gmail"),
        };

        assert_eq!(
            serde_json::to_value(&output).unwrap(),
            json!({
                "row": 89,
                "handle": "e2646df031cd876334410c4df14ebd86.1.89",
                "name": "Projets",
                "parent": null,
                "source": "google/gmail",
            })
        );
        let schema = output_schema::<PimdirMailboxCreateOutput>();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&Value::from("parent")), "{schema}");
    }
}
