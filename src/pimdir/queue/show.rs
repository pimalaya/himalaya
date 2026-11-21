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

use crate::pimdir::{
    client::{PimdirClient, row_not_found, store_reset},
    queue::list::PimdirQueueRow,
};

/// Show where one queue row stands, by the id `pimdir message add`,
/// `pimdir message send` or `pimdir mailbox create`, `rename` or `delete`
/// printed.
///
/// A row still queued prints as `pimdir queue list` prints it, then where
/// it stands. A row is `pending` until the sync engine applies it, `parked`
/// when the engine gave up on it (with why), `unconfirmed` when the engine
/// sent its request and cannot tell whether it took effect (a send or a
/// mailbox change that may have happened: it waits for the engine to
/// confirm it, or for a cancellation), and `applied` once done, a message
/// sent or a mailbox created, renamed or deleted included, with the id of
/// the message it acted on (the one an add created) as `itemId`. An applied
/// row lists the refusals standing on that message: a server that refused
/// for good the change the row led to, with its reason, held until the
/// message is changed again or removed. The store keeps what became of an
/// applied row for seven days, and while a refusal stands on its message,
/// under the mailbox it was queued on, or under none once that mailbox is
/// gone.
///
/// A row the account's queue does not hold, cancelled or applied long ago,
/// fails coded `row-not-found`; a handle another store answered, a copy
/// restored since, fails coded `store-reset`.
///
/// JSON output: the row as `pimdir queue list` prints it, `{"row",
/// "handle", "kind", "collection", "linkId", "itemId", "producer",
/// "queuedAt", "attempts", "unconfirmed", "error", "subject"}`, each `null`
/// that an applied row no longer carries, then `{"state", "appliedAt",
/// "refusals": [{"collection", "source", "reason", "refusedAt"}]}`.
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
        let handle = client.named(&self.id)?;
        let listed = client.listed_row(&handle, &status)?;
        printer.out(PimdirQueueShowOutput::new(&handle, status, listed)?)
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
    /// Sent, maybe in effect, awaiting confirmation or cancellation;
    /// `error` holds what the engine recorded.
    Unconfirmed,
    /// Applied or performed; `itemId` is the message it acted on, and
    /// `refusals` the servers that refused the change it led to.
    Applied,
}

/// The `pimdir queue show` output: the row `pimdir queue list` prints,
/// then what only a read carries.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueueShowOutput {
    /// The queue row id.
    pub row: i64,
    /// The row's handle, its id under the store's identity (pimdir STORAGE
    /// §4.5), a bare id given named under this store.
    pub handle: String,
    /// The row's action as the store holds it, `null` once applied.
    pub kind: Option<String>,
    /// The mailbox the row is queued on: `null` for an applied row whose
    /// mailbox is gone since.
    pub collection: Option<String>,
    /// The link id of the message the row creates, `null` when it creates
    /// none or is applied.
    pub link_id: Option<String>,
    /// The id of the message the row addresses or acted on (one an add
    /// created), as `envelope list` prints it; `null` when there is none.
    pub item_id: Option<String>,
    /// The process that staged the row, `null` once applied.
    pub producer: Option<String>,
    /// When the row was appended, `null` once applied.
    pub queued_at: Option<String>,
    /// How many times the sync engine tried the row, `null` once applied.
    pub attempts: Option<u64>,
    /// Whether the row's outcome is unknown.
    pub unconfirmed: bool,
    /// Why the row was parked, or what the sync engine recorded of an
    /// unconfirmed one.
    pub error: Option<String>,
    /// The subject of the body the row pins, else of the message it
    /// addresses, `null` when there is none to read or once applied.
    pub subject: Option<String>,
    /// Where it stands.
    pub state: PimdirQueueState,
    /// When the row was applied, RFC 3339.
    pub applied_at: Option<String>,
    /// The refusals standing on the message an applied row acted on, empty
    /// but for an applied row.
    pub refusals: Vec<PimdirQueueRefusal>,
}

/// A change the row led to that `source` refused for good (pimdir STORAGE
/// §15.4), held until the message changes again.
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
    /// Projects io-pimdir's status of the row `handle` names over
    /// `listed`, the row `pimdir queue list` prints while it is queued.
    ///
    /// A row neither queued nor known applied fails `row-not-found`, and
    /// one of another store `store-reset`.
    pub fn new(
        handle: &PimdirActionHandle,
        status: PimdirActionStatus,
        listed: Option<PimdirQueueRow>,
    ) -> Result<Self> {
        let row = Self {
            row: handle.id,
            handle: handle.to_string(),
            kind: None,
            collection: None,
            link_id: None,
            item_id: None,
            producer: None,
            queued_at: None,
            attempts: None,
            unconfirmed: false,
            error: None,
            subject: None,
            state: PimdirQueueState::Pending,
            applied_at: None,
            refusals: Vec::new(),
        };

        let row = match listed {
            Some(listed) => Self {
                kind: Some(listed.kind),
                collection: Some(listed.collection),
                link_id: listed.link_id,
                item_id: listed.item_id,
                producer: Some(listed.producer),
                queued_at: Some(listed.queued_at),
                attempts: Some(listed.attempts),
                unconfirmed: listed.unconfirmed,
                error: listed.error,
                subject: listed.subject,
                ..row
            },
            None => row,
        };

        let attempts = |attempts: i64| Some(u64::try_from(attempts).unwrap_or_default());

        Ok(match status {
            PimdirActionStatus::Pending {
                collection,
                kind,
                attempts: tried,
            } => Self {
                state: PimdirQueueState::Pending,
                collection: Some(collection),
                kind: Some(kind),
                attempts: attempts(tried),
                ..row
            },
            PimdirActionStatus::Parked {
                collection,
                kind,
                attempts: tried,
                error,
            } => Self {
                state: PimdirQueueState::Parked,
                collection: Some(collection),
                kind: Some(kind),
                attempts: attempts(tried),
                error: Some(error),
                ..row
            },
            PimdirActionStatus::Unconfirmed {
                collection,
                kind,
                attempts: tried,
                detail,
            } => Self {
                state: PimdirQueueState::Unconfirmed,
                collection: Some(collection),
                kind: Some(kind),
                attempts: attempts(tried),
                unconfirmed: true,
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
                item_id: seq.map(|seq| seq.to_string()),
                applied_at: Some(applied_at),
                refusals: refusals.into_iter().map(Into::into).collect(),
                ..row
            },
            PimdirActionStatus::Unknown => return Err(row_not_found(handle)),
            PimdirActionStatus::Reset => return Err(store_reset(handle)),
        })
    }
}

impl fmt::Display for PimdirQueueShowOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = &self.handle;
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
                if let Some(item) = &self.item_id {
                    write!(f, ", on message {}", sanitize::text(item))?;
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
        }
    }
}

#[cfg(test)]
mod tests {
    use io_pimdir::codec::PimdirStoreIdentity;
    use pimalaya_cli::error::code_of;

    use crate::{error::ErrorCode, pimdir::queue::show::*};

    const STORE: &str = "e2646df031cd876334410c4df14ebd86";

    /// The handle of row `id` of the store `STORE`.
    fn handle(id: i64) -> PimdirActionHandle {
        PimdirActionHandle {
            id,
            store: Some(PimdirStoreIdentity {
                store_id: STORE.into(),
                incarnation: 1,
            }),
        }
    }

    #[test]
    fn an_applied_add_prints_the_message_it_created() {
        let row = PimdirQueueShowOutput::new(
            &handle(12),
            PimdirActionStatus::Applied {
                applied_at: "2026-10-04T20:00:00Z".into(),
                collection: Some("imap/Drafts".into()),
                seq: Some(42),
                refusals: Vec::new(),
            },
            None,
        )
        .unwrap();
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "row": 12,
                "handle": format!("{STORE}.1.12"),
                "kind": null,
                "collection": "imap/Drafts",
                "linkId": null,
                "itemId": "42",
                "producer": null,
                "queuedAt": null,
                "attempts": null,
                "unconfirmed": false,
                "error": null,
                "subject": null,
                "state": "applied",
                "appliedAt": "2026-10-04T20:00:00Z",
                "refusals": [],
            })
        );
    }

    /// A row still queued prints the keys `pimdir queue list` prints for
    /// it, then where it stands.
    #[test]
    fn a_queued_row_prints_its_list_row() {
        let listed = PimdirQueueRow {
            row: 3,
            handle: format!("{STORE}.1.3"),
            kind: "set-flags".into(),
            collection: "imap/INBOX".into(),
            link_id: None,
            item_id: Some("7".into()),
            producer: "himalaya".into(),
            queued_at: "2026-11-15T09:00:00Z".into(),
            attempts: 0,
            unconfirmed: false,
            error: None,
            subject: Some("Report".into()),
        };
        let status = PimdirActionStatus::Pending {
            collection: "imap/INBOX".into(),
            kind: "set-flags".into(),
            attempts: 0,
        };

        let row = PimdirQueueShowOutput::new(&handle(3), status, Some(listed.clone())).unwrap();
        let json = serde_json::to_value(&row).unwrap();
        let listed = serde_json::to_value(&listed).unwrap();

        for (key, value) in listed.as_object().unwrap() {
            assert_eq!(&json[key], value, "{key}");
        }
        assert_eq!(json["state"], "pending");
    }

    /// An applied row shows a line per refusal with its source and reason
    /// (pimdir STORAGE §15.4), and the JSON lists them.
    #[test]
    fn an_applied_row_prints_the_refusals_on_its_message() {
        let row = PimdirQueueShowOutput::new(
            &handle(12),
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
            None,
        )
        .unwrap();

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
            format!(
                "Row {STORE}.1.12 was applied in msgraph/Inbox, on message 42\n\
             Refused by msgraph in msgraph/Inbox at 2026-11-15T20:05:00.000Z: \
             ErrorInvalidOperation\n"
            )
        );
    }

    /// A sent, unconfirmed intent (pimdir STORAGE §15.2) reads
    /// `unconfirmed` with what the performer recorded, never `parked`.
    #[test]
    fn an_unconfirmed_row_says_it_may_have_taken_effect() {
        let row = PimdirQueueShowOutput::new(
            &handle(9),
            PimdirActionStatus::Unconfirmed {
                collection: "imap/Sent".into(),
                kind: "submit".into(),
                attempts: 1,
                detail: "connection lost after DATA".into(),
            },
            None,
        )
        .unwrap();

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["state"], "unconfirmed");
        assert_eq!(json["unconfirmed"], true);
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
            &handle(4),
            PimdirActionStatus::Applied {
                applied_at: "2026-11-13T09:00:00Z".into(),
                collection: None,
                seq: None,
                refusals: Vec::new(),
            },
            None,
        )
        .unwrap();

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["state"], "applied");
        assert_eq!(json["collection"], serde_json::Value::Null);
        assert!(row.to_string().contains("deleted since"), "{row}");
    }

    /// A row the queue does not hold is no state but a failure, and a
    /// handle of another store fails apart.
    #[test]
    fn an_unknown_row_is_not_found() {
        let unknown = PimdirActionStatus::Unknown;
        let err = PimdirQueueShowOutput::new(&handle(7), unknown, None).unwrap_err();
        assert_eq!(code_of(&err), Some(ErrorCode::RowNotFound), "{err}");

        let reset = PimdirActionStatus::Reset;
        let err = PimdirQueueShowOutput::new(&handle(7), reset, None).unwrap_err();
        assert_eq!(code_of(&err), Some(ErrorCode::StoreReset), "{err}");
    }
}
