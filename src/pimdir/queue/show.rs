//! # pimdir queue show
//!
//! The `pimdir queue show` command, saying where one queue row stands.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{printer::Printer, table::sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::pimdir::client::PimdirClient;

/// Show where one queue row stands, by the id `pimdir message add`,
/// `pimdir message send` or `pimdir mailbox create` printed.
///
/// A row is `pending` until the sync engine applies it, `parked` when the
/// engine gave up on it (with why), and `gone` once the queue no longer
/// holds it: applied, or cancelled.
#[derive(Debug, Parser)]
pub struct PimdirQueueShowCommand {
    /// The queue row id.
    #[arg(value_name = "ROW")]
    pub id: i64,
}

impl PimdirQueueShowCommand {
    /// Looks the row up in the account's queue.
    pub fn execute(self, printer: &mut impl Printer, client: &mut PimdirClient) -> Result<()> {
        let output = match client.queue_row(self.id)? {
            Some(row) => PimdirQueueRow {
                queue_id: row.id,
                state: match row.error {
                    Some(_) => PimdirQueueState::Parked,
                    None => PimdirQueueState::Pending,
                },
                collection: Some(row.collection),
                kind: Some(row.kind),
                attempts: Some(row.attempts),
                error: row.error,
            },
            None => PimdirQueueRow {
                queue_id: self.id,
                state: PimdirQueueState::Gone,
                collection: None,
                kind: None,
                attempts: None,
                error: None,
            },
        };
        printer.out(output)
    }
}

/// Where a queue row stands.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PimdirQueueState {
    /// Waiting for the sync engine.
    Pending,
    /// Given up on by the sync engine; `error` says why.
    Parked,
    /// No longer in the queue: applied, or cancelled.
    Gone,
}

/// The `pimdir queue show` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueueRow {
    /// The queue row id.
    pub queue_id: i64,
    /// Where it stands.
    pub state: PimdirQueueState,
    /// The collection it is anchored on, while in the queue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    /// The action kind (`add`, `submit`…), while in the queue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Apply attempts so far, while in the queue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempts: Option<i64>,
    /// Why the row was parked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl fmt::Display for PimdirQueueRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = self.queue_id;
        match (self.state, &self.kind, &self.collection) {
            (PimdirQueueState::Gone, ..) => {
                write!(f, "Row {id} is no longer queued: applied or cancelled")
            }
            (state, Some(kind), Some(collection)) => {
                let state = match state {
                    PimdirQueueState::Parked => "parked",
                    _ => "pending",
                };
                let (kind, collection) = (sanitize(kind), sanitize(collection));
                write!(f, "Row {id} ({kind} in {collection}) is {state}")?;
                if let Some(error) = &self.error {
                    write!(f, ": {}", sanitize(error))?;
                }
                Ok(())
            }
            _ => write!(f, "Row {id}"),
        }
    }
}
