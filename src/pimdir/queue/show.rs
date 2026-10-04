//! # pimdir queue show
//!
//! The `pimdir queue show` command, saying where one queue row stands.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_pimdir::client::producer::PimdirActionStatus;
use pimalaya_cli::{printer::Printer, table::sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::pimdir::client::PimdirClient;

/// Show where one queue row stands, by the id `pimdir message add`,
/// `pimdir message send` or `pimdir mailbox create` printed.
///
/// A row is `pending` until the sync engine applies it, `parked` when the
/// engine gave up on it (with why), and `applied` once done, a message
/// sent or a mailbox created included, with the id of the message an add
/// created. The store keeps what became of an applied row for seven days;
/// past that, and for a cancelled row, the state is `unknown`.
#[derive(Debug, Parser)]
pub struct PimdirQueueShowCommand {
    /// The queue row id.
    #[arg(value_name = "ROW")]
    pub id: i64,
}

impl PimdirQueueShowCommand {
    /// Looks the row up in the account's queue and receipts.
    pub fn execute(self, printer: &mut impl Printer, client: &mut PimdirClient) -> Result<()> {
        let status = client.queue_row(self.id)?;
        printer.out(PimdirQueueRow::new(self.id, status))
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
    /// Applied or performed by the sync engine (a message sent, a mailbox
    /// created); `seq` is the message an add created.
    Applied,
    /// Neither queued nor known applied: cancelled, or applied long ago.
    Unknown,
}

/// The `pimdir queue show` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueueRow {
    /// The queue row id.
    pub queue_id: i64,
    /// Where it stands.
    pub state: PimdirQueueState,
    /// The collection it is anchored on, unless unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    /// The action kind (`add`, `submit`…), while queued.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Apply attempts so far, while queued.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempts: Option<i64>,
    /// Why the row was parked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// When the row was applied, RFC 3339.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_at: Option<String>,
    /// The id of the message an applied add created, in `collection`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<i64>,
}

impl PimdirQueueRow {
    /// Projects io-pimdir's status of row `queue_id`.
    pub fn new(queue_id: i64, status: PimdirActionStatus) -> Self {
        let row = Self {
            queue_id,
            state: PimdirQueueState::Unknown,
            collection: None,
            kind: None,
            attempts: None,
            error: None,
            applied_at: None,
            seq: None,
        };

        match status {
            PimdirActionStatus::Pending {
                collection,
                kind,
                attempts,
            } => Self {
                state: PimdirQueueState::Pending,
                collection: Some(collection),
                kind: Some(kind),
                attempts: Some(attempts),
                ..row
            },
            PimdirActionStatus::Parked {
                collection,
                kind,
                attempts,
                error,
            } => Self {
                state: PimdirQueueState::Parked,
                collection: Some(collection),
                kind: Some(kind),
                attempts: Some(attempts),
                error: Some(error),
                ..row
            },
            PimdirActionStatus::Applied {
                applied_at,
                collection,
                seq,
            } => Self {
                state: PimdirQueueState::Applied,
                collection: Some(collection),
                applied_at: Some(applied_at),
                seq,
                ..row
            },
            PimdirActionStatus::Unknown => row,
        }
    }
}

impl fmt::Display for PimdirQueueRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = self.queue_id;
        let collection = sanitize(self.collection.as_deref().unwrap_or_default());
        let kind = sanitize(self.kind.as_deref().unwrap_or_default());

        match self.state {
            PimdirQueueState::Pending => write!(f, "Row {id} ({kind} in {collection}) is pending"),
            PimdirQueueState::Parked => {
                let error = sanitize(self.error.as_deref().unwrap_or_default());
                write!(f, "Row {id} ({kind} in {collection}) is parked: {error}")
            }
            PimdirQueueState::Applied => {
                write!(f, "Row {id} was applied in {collection}")?;
                match self.seq {
                    Some(seq) => write!(f, ", creating message {seq}"),
                    None => Ok(()),
                }
            }
            PimdirQueueState::Unknown => write!(
                f,
                "Row {id} is neither queued nor known applied: cancelled, or applied long ago"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_applied_add_prints_its_seq() {
        let row = PimdirQueueRow::new(
            12,
            PimdirActionStatus::Applied {
                applied_at: "2026-10-04T20:00:00Z".into(),
                collection: "imap/Drafts".into(),
                seq: Some(42),
            },
        );
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "queueId": 12,
                "state": "applied",
                "collection": "imap/Drafts",
                "appliedAt": "2026-10-04T20:00:00Z",
                "seq": 42,
            })
        );
    }

    #[test]
    fn an_unknown_row_prints_its_id_and_state_only() {
        let json =
            serde_json::to_value(PimdirQueueRow::new(7, PimdirActionStatus::Unknown)).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "queueId": 7, "state": "unknown" })
        );
    }
}
