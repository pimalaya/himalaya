//! # pimdir queue show
//!
//! The `pimdir queue show` command, saying where one queue row stands.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_pimdir::{
    client::producer::{PimdirActionStatus, PimdirReceiptRefusal},
    codec::PimdirActionHandle,
};
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
/// message it acted on (the one an add created). An applied row lists the
/// refusals standing on that message: a server that refused for good the
/// change the row led to, with its reason, held until the message is
/// changed again or removed. The store keeps what became of an applied row
/// for seven days, and while a refusal stands on its message, under the
/// mailbox it was queued on, or under none once that mailbox is gone; past
/// that, and for a cancelled row, the state is `unknown`.
///
/// JSON output: `{"row", "state", "collection", "kind", "attempts",
/// "error", "appliedAt", "seq", "refusals": [{"collection", "source",
/// "reason", "refusedAt"}]}`.
#[derive(Debug, Parser)]
pub struct PimdirQueueShowCommand {
    /// The handle of the queue row, as a receipt prints it, or its bare
    /// id, as `pimdir queue list` prints it.
    #[arg(value_name = "ROW")]
    pub id: PimdirActionHandle,
}

impl PimdirQueueShowCommand {
    /// Looks the row up in the account's queue and receipts.
    pub fn execute(self, printer: &mut impl Printer, client: &mut PimdirClient) -> Result<()> {
        let status = client.queue_row(&self.id)?;
        printer.out(PimdirQueueShowOutput::new(self.id.to_string(), status))
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
    /// created, renamed or deleted); `seq` is the message it acted on, and
    /// `refusals` the servers that refused the change it led to.
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
    /// The queue row, as it was given.
    #[serde(rename = "row")]
    pub queue_id: String,
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
    /// The id of the message an applied row acted on (the one an add
    /// created), as `envelope list` prints it.
    pub seq: Option<i64>,
    /// The refusals standing on that message, empty but for an applied
    /// row.
    pub refusals: Vec<PimdirQueueRefusal>,
}

/// A refusal standing on the message an applied row acted on (pimdir
/// STORAGE §15.4): the server of `source` refused for good the change the
/// row led to, and the sync engine holds it until the message changes
/// again.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueueRefusal {
    /// The mailbox filing the message, the target of a move.
    pub collection: String,
    /// The source whose server refused the change.
    pub source: String,
    /// The text the server gave.
    pub reason: String,
    /// When the refusal was recorded, RFC 3339.
    pub refused_at: String,
}

impl From<PimdirReceiptRefusal> for PimdirQueueRefusal {
    fn from(refusal: PimdirReceiptRefusal) -> Self {
        Self {
            collection: refusal.collection,
            source: refusal.source,
            reason: refusal.reason,
            refused_at: refusal.refused_at,
        }
    }
}

impl PimdirQueueShowOutput {
    /// Projects io-pimdir's status of row `queue_id`.
    pub fn new(queue_id: String, status: PimdirActionStatus) -> Self {
        let row = Self {
            queue_id,
            state: PimdirQueueState::Unknown,
            collection: None,
            kind: None,
            attempts: None,
            error: None,
            applied_at: None,
            seq: None,
            refusals: Vec::new(),
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
                refusals,
            } => Self {
                state: PimdirQueueState::Applied,
                collection,
                applied_at: Some(applied_at),
                seq,
                refusals: refusals.into_iter().map(Into::into).collect(),
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
        let id = &self.queue_id;
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
                    write!(f, ", on message {seq}")?;
                }
                writeln!(f)?;
                for refusal in &self.refusals {
                    writeln!(
                        f,
                        "Refused by {} in {} at {}: {}",
                        sanitize::text(&refusal.source),
                        sanitize::text(&refusal.collection),
                        sanitize::text(&refusal.refused_at),
                        sanitize::text(&refusal.reason),
                    )?;
                }
                Ok(())
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
            "12".into(),
            PimdirActionStatus::Applied {
                applied_at: "2026-10-04T20:00:00Z".into(),
                collection: Some("imap/Drafts".into()),
                seq: Some(42),
                refusals: Vec::new(),
            },
        );
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "row": "12",
                "state": "applied",
                "collection": "imap/Drafts",
                "kind": null,
                "attempts": null,
                "error": null,
                "appliedAt": "2026-10-04T20:00:00Z",
                "seq": 42,
                "refusals": [],
            })
        );
    }

    /// An applied row whose change a server refused for good shows applied,
    /// then a line per refusal with its source and reason (pimdir STORAGE
    /// §15.4), and the JSON lists them.
    #[test]
    fn an_applied_row_prints_the_refusals_on_its_message() {
        let row = PimdirQueueShowOutput::new(
            "12".into(),
            PimdirActionStatus::Applied {
                applied_at: "2026-11-15T20:00:00.000Z".into(),
                collection: Some("msgraph/Inbox".into()),
                seq: Some(42),
                refusals: vec![PimdirReceiptRefusal {
                    collection: "msgraph/Inbox".into(),
                    source: "msgraph".into(),
                    reason: "ErrorInvalidOperation".into(),
                    refused_at: "2026-11-15T20:05:00.000Z".into(),
                }],
            },
        );

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["state"], "applied");
        assert_eq!(
            json["refusals"],
            serde_json::json!([{
                "collection": "msgraph/Inbox",
                "source": "msgraph",
                "reason": "ErrorInvalidOperation",
                "refusedAt": "2026-11-15T20:05:00.000Z",
            }])
        );
        assert_eq!(
            row.to_string(),
            "Row 12 was applied in msgraph/Inbox, on message 42\n\
             Refused by msgraph in msgraph/Inbox at 2026-11-15T20:05:00.000Z: \
             ErrorInvalidOperation\n"
        );
    }

    /// An intent its performer sent and could not confirm is no failure
    /// and no success (pimdir STORAGE §15.2): it reads `unconfirmed`, with
    /// what the performer recorded, never `parked`.
    #[test]
    fn an_unconfirmed_row_says_it_may_have_taken_effect() {
        let row = PimdirQueueShowOutput::new(
            "9".into(),
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
            "4".into(),
            PimdirActionStatus::Applied {
                applied_at: "2026-11-13T09:00:00Z".into(),
                collection: None,
                seq: None,
                refusals: Vec::new(),
            },
        );

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["state"], "applied");
        assert_eq!(json["collection"], serde_json::Value::Null);
        assert!(row.to_string().contains("deleted since"), "{row}");
    }

    #[test]
    fn an_unknown_row_prints_its_id_and_state() {
        let json = serde_json::to_value(PimdirQueueShowOutput::new(
            "7".into(),
            PimdirActionStatus::Unknown,
        ))
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "row": "7",
                "state": "unknown",
                "collection": null,
                "kind": null,
                "attempts": null,
                "error": null,
                "appliedAt": null,
                "seq": null,
                "refusals": [],
            })
        );
    }
}
