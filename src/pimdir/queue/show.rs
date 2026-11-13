//! # pimdir queue show
//!
//! The `pimdir queue show` command, saying where one queue row stands.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_pimdir::client::producer::PimdirActionStatus;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::pimdir::client::PimdirClient;

/// Show where one queue row stands, by the id `pimdir message add`,
/// `pimdir message send` or `pimdir mailbox create`, `rename` or `delete`
/// printed.
///
/// A row is `pending` until the sync engine applies it, `parked` when the
/// engine gave up on it (with why), `unconfirmed` when the engine sent its
/// request and cannot tell whether it took effect (a send or a mailbox
/// change that may have happened: it waits for the engine to confirm it,
/// or for a cancellation), and `applied` once done, a message sent or a
/// mailbox created, renamed or deleted included, with the id of the
/// message an add created. The store keeps what became of an applied row
/// for seven days, under the mailbox it was queued on, or under none once
/// that mailbox is gone; past that, and for a cancelled row, the state is
/// `unknown`.
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
        printer.out(PimdirQueueShowOutput::new(self.id, status))
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
    /// Sent by the sync engine, which cannot tell whether it took effect:
    /// it may have, and waits for the engine to confirm it or for a
    /// cancellation; `error` holds what the engine recorded.
    Unconfirmed,
    /// Applied or performed by the sync engine (a message sent, a mailbox
    /// created, renamed or deleted); `seq` is the message an add created.
    Applied,
    /// Neither queued nor known applied: cancelled, or applied long ago.
    Unknown,
    /// The store is not the one the row was queued in, a copy restored
    /// since, where the id may name another row: nothing is read of it.
    Reset,
}

/// The `pimdir queue show` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueueShowOutput {
    /// The queue row id.
    #[serde(rename = "row")]
    pub queue_id: i64,
    /// Where it stands.
    pub state: PimdirQueueState,
    /// The collection it is anchored on: `null` when unknown, and for an
    /// applied row whose mailbox is gone since.
    pub collection: Option<String>,
    /// The action kind (`add`, `submit`…), while queued.
    pub kind: Option<String>,
    /// Apply attempts so far, while queued.
    pub attempts: Option<i64>,
    /// Why the row was parked, or what the sync engine recorded of an
    /// unconfirmed one.
    pub error: Option<String>,
    /// When the row was applied, RFC 3339.
    pub applied_at: Option<String>,
    /// The id of the message an applied add created, in `collection`.
    pub seq: Option<i64>,
}

impl PimdirQueueShowOutput {
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
            PimdirActionStatus::Unconfirmed {
                collection,
                kind,
                attempts,
                detail,
            } => Self {
                state: PimdirQueueState::Unconfirmed,
                collection: Some(collection),
                kind: Some(kind),
                attempts: Some(attempts),
                error: Some(detail),
                ..row
            },
            PimdirActionStatus::Applied {
                applied_at,
                collection,
                seq,
            } => Self {
                state: PimdirQueueState::Applied,
                collection,
                applied_at: Some(applied_at),
                seq,
                ..row
            },
            PimdirActionStatus::Unknown => row,
            PimdirActionStatus::Reset => Self {
                state: PimdirQueueState::Reset,
                ..row
            },
        }
    }
}

impl fmt::Display for PimdirQueueShowOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = self.queue_id;
        let collection = sanitize::text(self.collection.as_deref().unwrap_or_default());
        let kind = sanitize::text(self.kind.as_deref().unwrap_or_default());

        match self.state {
            PimdirQueueState::Pending => {
                writeln!(f, "Row {id} ({kind} in {collection}) is pending")
            }
            PimdirQueueState::Parked => {
                let error = sanitize::text(self.error.as_deref().unwrap_or_default());
                writeln!(f, "Row {id} ({kind} in {collection}) is parked: {error}")
            }
            PimdirQueueState::Unconfirmed => {
                let detail = sanitize::text(self.error.as_deref().unwrap_or_default());
                writeln!(
                    f,
                    "Row {id} ({kind} in {collection}) is unconfirmed, sent and maybe \
                     performed: {detail}"
                )
            }
            PimdirQueueState::Applied => {
                match self.collection {
                    Some(_) => write!(f, "Row {id} was applied in {collection}")?,
                    None => write!(f, "Row {id} was applied in a mailbox deleted since")?,
                }
                if let Some(seq) = self.seq {
                    write!(f, ", creating message {seq}")?;
                }
                writeln!(f)
            }
            PimdirQueueState::Unknown => writeln!(
                f,
                "Row {id} is neither queued nor known applied: cancelled, or applied long ago"
            ),
            PimdirQueueState::Reset => writeln!(
                f,
                "Row {id} was queued in another store: a copy restored since, where this id \
                 may name another row"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_applied_add_prints_its_seq() {
        let row = PimdirQueueShowOutput::new(
            12,
            PimdirActionStatus::Applied {
                applied_at: "2026-10-04T20:00:00Z".into(),
                collection: Some("imap/Drafts".into()),
                seq: Some(42),
            },
        );
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "row": 12,
                "state": "applied",
                "collection": "imap/Drafts",
                "kind": null,
                "attempts": null,
                "error": null,
                "appliedAt": "2026-10-04T20:00:00Z",
                "seq": 42,
            })
        );
    }

    /// An intent its performer sent and could not confirm is no failure
    /// and no success (pimdir STORAGE §15.2): it reads `unconfirmed`, with
    /// what the performer recorded, never `parked`.
    #[test]
    fn an_unconfirmed_row_says_it_may_have_taken_effect() {
        let row = PimdirQueueShowOutput::new(
            9,
            PimdirActionStatus::Unconfirmed {
                collection: "imap/Sent".into(),
                kind: "submit".into(),
                attempts: 1,
                detail: "connection lost after DATA".into(),
            },
        );

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["state"], "unconfirmed");
        assert_eq!(json["kind"], "submit");
        assert_eq!(json["error"], "connection lost after DATA");

        let line = row.to_string();
        assert!(line.contains("unconfirmed"), "{line}");
        assert!(line.contains("maybe performed"), "{line}");
        assert!(line.contains("connection lost after DATA"), "{line}");
    }

    /// A row applied on a mailbox deleted since keeps its receipt and
    /// names no collection (pimdir STORAGE §15.4).
    #[test]
    fn an_applied_row_of_a_deleted_mailbox_names_no_collection() {
        let row = PimdirQueueShowOutput::new(
            4,
            PimdirActionStatus::Applied {
                applied_at: "2026-11-13T09:00:00Z".into(),
                collection: None,
                seq: None,
            },
        );

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["state"], "applied");
        assert_eq!(json["collection"], serde_json::Value::Null);
        assert!(row.to_string().contains("deleted since"), "{row}");
    }

    #[test]
    fn an_unknown_row_prints_its_id_and_state() {
        let json = serde_json::to_value(PimdirQueueShowOutput::new(7, PimdirActionStatus::Unknown))
            .unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "row": 7,
                "state": "unknown",
                "collection": null,
                "kind": null,
                "attempts": null,
                "error": null,
                "appliedAt": null,
                "seq": null,
            })
        );
    }
}
