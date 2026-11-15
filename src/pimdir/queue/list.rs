//! # pimdir queue list
//!
//! The `pimdir queue list` command, listing the creations and sends staged
//! in one mailbox and the rows parked there.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, backend::Backend, pimdir::client::PimdirClient,
    shared::mailbox::arg::MailboxArg,
};

/// List the messages staged for creation or sending in a mailbox.
///
/// A saved message waits in the queue until the sync engine applies it and
/// has no id until then, so `envelope list` cannot show it. A sent message
/// waits there until the store's owner sends it, filed under the mailbox it
/// was saved to or the account's sent alias. This is where both show: the
/// row id to cancel one by, when it was queued, its kind (`add` or
/// `submit`) and its subject.
///
/// Staged flags, moves and deletions need no such view, addressing messages
/// that already exist. A row of any kind the sync engine parked, refusing to
/// apply it without an operator, is listed `parked` with why: cancel it, or
/// fix what parked it. A move or a deletion of a message a sync left in
/// conflict parks so, listed in the mailbox the message stays in. One the
/// engine sent without learning its outcome is listed `unconfirmed`
/// instead: it may have taken effect.
///
/// JSON output: `{"messages": [<row>], "parked": [<row>]}`, each row,
/// pending or parked, `{"row", "kind", "collection", "linkId", "itemId",
/// "producer", "queuedAt", "attempts", "unconfirmed", "error",
/// "subject"}`, as every Pimalaya program prints a queue row.
#[derive(Debug, Parser)]
pub struct PimdirQueueListCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
}

impl PimdirQueueListCommand {
    /// Lists the creations and sends staged in the mailbox and the rows
    /// parked there, and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account);
        let messages = client.queued_rows(&mailbox)?;
        let parked = client.parked_rows(&mailbox)?;

        printer.out(PimdirQueueListOutput {
            account: account.name().to_string(),
            mailbox,
            messages,
            parked,
        })
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
    pub attempts: i64,
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
    /// The mailbox listed, its alias resolved.
    #[serde(skip)]
    pub mailbox: String,
    /// The creations and sends staged in the mailbox.
    pub messages: Vec<PimdirQueueRow>,
    /// The mailbox's parked rows, any kind.
    pub parked: Vec<PimdirQueueRow>,
}

impl fmt::Display for PimdirQueueListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Pimdir.as_str()));
        listing.head("Mailbox", &self.mailbox);

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

        if !listing.items.rows.is_empty() {
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

    use super::{PimdirQueueListOutput, PimdirQueueRow};

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
            mailbox: "imap/Sent".into(),
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
    }
}
