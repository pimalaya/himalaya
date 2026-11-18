//! # pimdir queue list
//!
//! The `pimdir queue list` command, listing the creations and sends staged
//! in the account, the rows parked there and the refusals standing, or
//! those of one mailbox.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, backend::Backend, pimdir::client::PimdirClient};

/// List the messages staged for creation or sending in the account.
///
/// A saved message waits in the queue until the sync engine applies it and
/// has no id until then, so `envelope list` cannot show it. A sent message
/// waits there until the store's owner sends it, filed under the mailbox it
/// was saved to or the account's sent alias. This is where both show: the
/// row id to cancel one by, when it was queued, its kind (`add` or
/// `submit`) and its subject. The whole account is listed, mailbox by
/// mailbox, unless `--mailbox` names one.
///
/// Staged flags, moves and deletions need no such view, addressing messages
/// that already exist. A row of any kind the sync engine parked, refusing to
/// apply it without an operator, is listed `parked` with why: cancel it, or
/// fix what parked it. A move or a deletion of a message a sync left in
/// conflict parks so, listed in the mailbox the message stays in. One the
/// engine sent without learning its outcome is listed `unconfirmed`
/// instead: it may have taken effect. A message a server refused for good
/// the change it was synced with is listed `refused` last, with its source
/// and reason: the store applied the change, and the sync engine holds it
/// until the message changes again or is removed (pimdir STORAGE §15.4).
///
/// JSON output: `{"messages": [<row>], "parked": [<row>], "refused":
/// [{"collection", "itemId", "source", "reason", "refusedAt"}]}`, each row,
/// pending or parked, `{"row", "kind", "collection", "linkId", "itemId",
/// "producer", "queuedAt", "attempts", "unconfirmed", "error",
/// "subject"}`, as every Pimalaya program prints a queue row.
#[derive(Debug, Parser)]
pub struct PimdirQueueListCommand {
    /// The one mailbox to list, a name, alias, role or id; the whole
    /// account when omitted.
    #[arg(short = 'm', long = "mailbox", value_name = "NAME")]
    pub mailbox: Option<String>,
}

impl PimdirQueueListCommand {
    /// Lists the creations and sends staged in the account, or in the one
    /// mailbox named, the rows parked there and the refusals standing,
    /// and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let mailboxes = match &self.mailbox {
            Some(mailbox) => vec![account.resolve_mailbox(mailbox).to_string()],
            None => client.mail_collection_ids()?,
        };

        let mut output = PimdirQueueListOutput {
            account: account.name().to_string(),
            mailbox: self.mailbox.as_ref().and(mailboxes.first().cloned()),
            messages: Vec::new(),
            parked: Vec::new(),
            refused: Vec::new(),
        };

        for mailbox in &mailboxes {
            output.messages.extend(client.queued_rows(mailbox)?);
            output.parked.extend(client.parked_rows(mailbox)?);
            output.refused.extend(client.refused_rows(mailbox)?);
        }

        printer.out(output)
    }
}

/// One row of the store's queue, pending or parked, in the one shape
/// every Pimalaya program prints it (pimalaya-cli's output contract, "A
/// queue row keeps one shape"), every key printed.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueueRow {
    /// The queue row id, which `pimdir queue show` and `cancel` take. It
    /// names a queued action, not a message: a creation has no id until
    /// the sync engine applies it, and gets a different one then.
    pub row: i64,
    /// The row's action as the store holds it, never re-spelled: `add`,
    /// `submit`, `set-flags`, `move`, `remove`, `collection-create`, ...
    pub kind: String,
    /// The mailbox the row is queued on, as `mailbox list` prints its id.
    pub collection: String,
    /// The link id of the message the row creates, the bare `Message-ID`
    /// `message add` printed, `null` when the row creates none.
    pub link_id: Option<String>,
    /// The id of the stored message the row addresses (a flag change, a
    /// move, a copy, a removal), as `envelope list` prints it, `null` when
    /// it addresses none, a creation among them.
    pub item_id: Option<String>,
    /// The process that staged the row.
    pub producer: String,
    /// When the row was appended, stamped by the store's own clock.
    pub queued_at: String,
    /// How many times the sync engine tried the row.
    pub attempts: u64,
    /// Whether the row's outcome is unknown: the engine sent its request
    /// and cannot tell whether it took effect, so it may have.
    pub unconfirmed: bool,
    /// Why the row was parked, as the store records it, an unconfirmed
    /// row's beginning `unconfirmed: `; `null` while the row is pending.
    pub error: Option<String>,
    /// The subject of the body the row pins, else of the message it
    /// addresses, `null` when there is none to read.
    pub subject: Option<String>,
}

/// The `pimdir queue list` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueueListOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The one mailbox listed, its alias resolved, `None` for the whole
    /// account.
    #[serde(skip)]
    pub mailbox: Option<String>,
    /// The creations and sends staged, mailbox by mailbox.
    pub messages: Vec<PimdirQueueRow>,
    /// The parked rows, any kind, mailbox by mailbox.
    pub parked: Vec<PimdirQueueRow>,
    /// The refusals standing, per message and source.
    pub refused: Vec<PimdirQueueRefused>,
}

/// A refusal standing on a message of the mailbox (pimdir STORAGE §15.4):
/// the server of `source` refused for good a change the store applied, and
/// the sync engine holds it until the message changes again or is removed.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueueRefused {
    /// The mailbox holding the message, as `mailbox list` prints its id.
    pub collection: String,
    /// The message, as `envelope list` prints its id.
    pub item_id: String,
    /// The source whose server refused the change.
    pub source: String,
    /// The text the server gave.
    pub reason: String,
    /// When the refusal was recorded, RFC 3339.
    pub refused_at: String,
}

impl fmt::Display for PimdirQueueListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Pimdir.as_str()));
        if let Some(mailbox) = &self.mailbox {
            listing.head("Mailbox", mailbox);
        }

        for queued in &self.messages {
            let what = queued.subject.as_deref().or(queued.item_id.as_deref());
            listing.items.push(cells(queued, "queued", what));
        }

        for parked in &self.parked {
            let state = match parked.unconfirmed {
                true => "unconfirmed",
                false => "parked",
            };
            listing
                .items
                .push(cells(parked, state, parked.error.as_deref()));
        }

        for refused in &self.refused {
            let reason = format!("{}: {}", refused.source, refused.reason);
            listing.items.push([
                String::new(),
                String::from("refused"),
                sanitize::text(&refused.refused_at).into_owned(),
                sanitize::text(&refused.item_id).into_owned(),
                sanitize::text(&reason).into_owned(),
            ]);
        }

        if !self.messages.is_empty() || !self.parked.is_empty() {
            let cancel = String::from("himalaya pimdir queue cancel <ROW>");
            listing.header.push(("Cancel", cancel));
        }

        write!(f, "{listing}")
    }
}

/// The text cells of `row`: its id, its `state`, when it was queued, its
/// kind, then `last`.
fn cells(row: &PimdirQueueRow, state: &str, last: Option<&str>) -> [String; 5] {
    [
        row.row.to_string(),
        String::from(state),
        sanitize::text(&row.queued_at).into_owned(),
        sanitize::text(&row.kind).into_owned(),
        sanitize::text(last.unwrap_or_default()).into_owned(),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{PimdirQueueListOutput, PimdirQueueRefused, PimdirQueueRow};

    /// A pending row of `kind` in `imap/Sent`.
    fn row(row: i64, kind: &str) -> PimdirQueueRow {
        PimdirQueueRow {
            row,
            kind: kind.into(),
            collection: "imap/Sent".into(),
            link_id: None,
            item_id: None,
            producer: "himalaya".into(),
            queued_at: "2026-11-15T09:00:00Z".into(),
            attempts: 0,
            unconfirmed: false,
            error: None,
            subject: None,
        }
    }

    /// A pending and a parked row print in one shape, every key present,
    /// as every Pimalaya program prints them, and one line each under the
    /// way to cancel one.
    #[test]
    fn a_pending_and_a_parked_row_keep_one_shape() {
        let output = PimdirQueueListOutput {
            account: "perso".into(),
            mailbox: Some("imap/Sent".into()),
            messages: vec![
                PimdirQueueRow {
                    link_id: Some("d1@x.org".into()),
                    subject: Some("hi".into()),
                    ..row(3, "add")
                },
                row(4, "submit"),
            ],
            parked: vec![PimdirQueueRow {
                item_id: Some("7".into()),
                attempts: 1,
                unconfirmed: true,
                error: Some("unconfirmed: timed out".into()),
                subject: Some("Report".into()),
                ..row(2, "move")
            }],
            refused: vec![PimdirQueueRefused {
                collection: "imap/Sent".into(),
                item_id: "9".into(),
                source: "imap".into(),
                reason: "NO [CANNOT] refused".into(),
                refused_at: "2026-11-15T09:10:00.000Z".into(),
            }],
        };

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(
            json["messages"][1],
            json!({
                "row": 4,
                "kind": "submit",
                "collection": "imap/Sent",
                "linkId": null,
                "itemId": null,
                "producer": "himalaya",
                "queuedAt": "2026-11-15T09:00:00Z",
                "attempts": 0,
                "unconfirmed": false,
                "error": null,
                "subject": null,
            })
        );
        let parked = json["parked"][0].as_object().unwrap();
        let keys: Vec<&str> = parked.keys().map(String::as_str).collect();
        let pending = json["messages"][0].as_object().unwrap();
        assert_eq!(keys, pending.keys().map(String::as_str).collect::<Vec<_>>());
        assert_eq!(parked["itemId"], "7");

        let text = output.to_string();
        assert!(text.contains("Cancel:"), "{text}");
        let lines: Vec<&str> = text.lines().skip_while(|l| !l.is_empty()).collect();
        assert!(
            lines[1].starts_with("3 ") && lines[1].contains("add"),
            "{text}"
        );
        assert!(lines[1].ends_with("hi"), "{text}");
        assert!(lines[3].contains("unconfirmed"), "{text}");
        assert!(lines[3].ends_with("unconfirmed: timed out"), "{text}");
        assert!(lines[4].contains("refused"), "{text}");
        assert!(lines[4].ends_with("imap: NO [CANNOT] refused"), "{text}");
        assert_eq!(json["refused"][0]["itemId"], "9");
        assert_eq!(json["refused"][0]["refusedAt"], "2026-11-15T09:10:00.000Z");
    }

    /// A row's `attempts` is an unsigned count, as the family's queue row
    /// declares it.
    #[test]
    fn a_row_counts_its_attempts_from_zero() {
        let schema = schemars::schema_for!(PimdirQueueRow);
        let attempts = &schema.as_value()["properties"]["attempts"];
        assert_eq!(attempts["format"], "uint64", "{attempts}");
        assert_eq!(attempts["minimum"], 0, "{attempts}");
    }
}
