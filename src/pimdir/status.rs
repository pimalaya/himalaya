//! # pimdir status
//!
//! The `pimdir status` command, reporting what the local pimdir store
//! holds of the account.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_pimdir::codec;
use pimalaya_cli::{columns::Columns, listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, backend::Backend, pimdir::client::PimdirClient};

/// Report what the local pimdir store holds of the account.
///
/// Shows the account the store groups this account's mail under, every
/// account it groups collections under, one row per mailbox with how many
/// of its messages carry a local body and how many creations and sends are
/// still queued, every write the sync engine parked in those mailboxes, and
/// every message a server refused for good.
///
/// A mailbox whose messages list but do not read is waiting on a sync to
/// download them, and this is where to see that coming. A parked write
/// never lands: it is shown by its queue row handle, which `pimdir queue
/// cancel` cancels it by, `pimdir queue list` listing every row. A refused
/// message was changed in the store, but a server refused for good the
/// change it led to: the sync engine holds that change, pushing it again
/// only once the message changes again or is removed (pimdir STORAGE
/// §15.4).
///
/// JSON output: `{"account", "accounts", "mailboxes": [{"id", "name",
/// "total", "hydrated", "queued"}], "parked": [{"row", "handle",
/// "mailboxId", "kind", "error"}], "refused": [{"mailboxId", "itemId",
/// "source", "reason", "refusedAt"}]}`, `kind` as the queue spells it.
#[derive(Debug, Parser)]
pub struct PimdirStatusCommand;

impl PimdirStatusCommand {
    /// Reads the report and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let mut status = client.status()?;
        status.name = account.name().to_string();
        printer.out(status)
    }
}

/// What a store holds of the account, as `pimdir status` reports it.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirStatusOutput {
    /// The name of the Himalaya account read.
    #[serde(skip)]
    pub name: String,
    /// The account the store groups this account's mail under, `null` in
    /// a store grouping none.
    pub account: Option<String>,
    /// Every account the store groups collections under.
    pub accounts: Vec<String>,
    /// One row per mailbox of the account, by id.
    pub mailboxes: Vec<PimdirMailboxStatus>,
    /// Every action the sync engine parked in those mailboxes, in the
    /// order it was queued.
    pub parked: Vec<PimdirParkedStatus>,
    /// Every refusal standing in those mailboxes, per message and source.
    pub refused: Vec<PimdirRefusedStatus>,
}

/// One mailbox's row in a [`PimdirStatusOutput`].
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailboxStatus {
    /// The mailbox, as `mailbox list` prints its id.
    pub id: String,
    /// The mailbox's name, its id when it has none.
    pub name: String,
    /// How many live messages it holds.
    pub total: u64,
    /// How many of those carry a local body.
    pub hydrated: u64,
    /// How many creations and sends are queued on it: they carry no id
    /// until the sync engine applies them, so they list nowhere else.
    pub queued: u64,
}

/// One parked action in a [`PimdirStatusOutput`].
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirParkedStatus {
    /// The queue row id, as `pimdir queue list` prints it.
    pub row: i64,
    /// The row's handle, its id under the store's identity (pimdir STORAGE
    /// §4.5), which `pimdir queue show` and `cancel` take and check.
    pub handle: String,
    /// The mailbox the action is queued on, as `mailbox list` prints its
    /// id.
    pub mailbox_id: String,
    /// The action as the queue spells it: `add`, `submit`, `set-flags`,
    /// `move`, ...
    pub kind: String,
    /// Why the sync engine parked it, an unconfirmed row's beginning
    /// `unconfirmed: `.
    pub error: String,
}

/// One refused message in a [`PimdirStatusOutput`] (pimdir STORAGE §15.4).
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirRefusedStatus {
    /// The mailbox holding the message, as `mailbox list` prints its id.
    pub mailbox_id: String,
    /// The message, as `envelope list` prints its id.
    pub item_id: String,
    /// The source whose server refused the change.
    pub source: String,
    /// The text the server gave.
    pub reason: String,
    /// When the refusal was recorded, RFC 3339.
    pub refused_at: String,
}

impl fmt::Display for PimdirStatusOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.name, Some(Backend::Pimdir.as_str()));

        match &self.account {
            Some(account) => listing.head("Store account", account),
            None => listing.head("Store account", "none, the store groups no account"),
        }

        match self.accounts.as_slice() {
            [] => listing.head("Grouped accounts", "none"),
            accounts => listing.head("Grouped accounts", &accounts.join(", ")),
        }

        listing.empty = Some("No mailbox");

        for mailbox in &self.mailboxes {
            listing.items.push([
                sanitize::text(&mailbox.id).into_owned(),
                format!("{}/{}", mailbox.hydrated, mailbox.total),
                format!("{} queued", mailbox.queued),
                sanitize::text(&mailbox.name).into_owned(),
            ]);
        }

        write!(f, "{listing}")?;

        if !self.parked.is_empty() {
            let mut parked = Columns::default();

            for row in &self.parked {
                let state = match codec::unconfirmed(&row.error) {
                    Some(_) => "unconfirmed",
                    None => "parked",
                };

                parked.push([
                    sanitize::text(&row.handle).into_owned(),
                    String::from(state),
                    sanitize::text(&row.kind).into_owned(),
                    sanitize::text(&row.mailbox_id).into_owned(),
                    sanitize::text(&row.error).into_owned(),
                ]);
            }

            write!(f, "\n{parked}")?;
        }

        if !self.refused.is_empty() {
            let mut refused = Columns::default();

            for row in &self.refused {
                refused.push([
                    sanitize::text(&row.item_id).into_owned(),
                    String::from("refused"),
                    sanitize::text(&row.source).into_owned(),
                    sanitize::text(&row.mailbox_id).into_owned(),
                    sanitize::text(&row.reason).into_owned(),
                ]);
            }

            write!(f, "\n{refused}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const STORE: &str = "e2646df031cd876334410c4df14ebd86";

    /// The report prints the twins' keys, a parked row by its row and
    /// handle, a mailbox under `mailboxId` in parked and refused rows; the
    /// text lists each block after the mailboxes.
    #[test]
    fn a_status_prints_mailboxes_parked_rows_and_refusals() {
        let output = PimdirStatusOutput {
            name: "work".into(),
            account: Some("work".into()),
            accounts: vec!["perso".into(), "work".into()],
            mailboxes: vec![PimdirMailboxStatus {
                id: "imap/INBOX".into(),
                name: "INBOX".into(),
                total: 40,
                hydrated: 12,
                queued: 1,
            }],
            parked: vec![PimdirParkedStatus {
                row: 3,
                handle: format!("{STORE}.1.3"),
                mailbox_id: "imap/INBOX".into(),
                kind: "move".into(),
                error: "conflict".into(),
            }],
            refused: vec![PimdirRefusedStatus {
                mailbox_id: "imap/INBOX".into(),
                item_id: "9".into(),
                source: "imap".into(),
                reason: "NO [CANNOT] refused".into(),
                refused_at: "2026-11-19T09:10:00.000Z".into(),
            }],
        };

        assert_eq!(
            serde_json::to_value(&output).unwrap(),
            json!({
                "account": "work",
                "accounts": ["perso", "work"],
                "mailboxes": [{
                    "id": "imap/INBOX",
                    "name": "INBOX",
                    "total": 40,
                    "hydrated": 12,
                    "queued": 1,
                }],
                "parked": [{
                    "row": 3,
                    "handle": format!("{STORE}.1.3"),
                    "mailboxId": "imap/INBOX",
                    "kind": "move",
                    "error": "conflict",
                }],
                "refused": [{
                    "mailboxId": "imap/INBOX",
                    "itemId": "9",
                    "source": "imap",
                    "reason": "NO [CANNOT] refused",
                    "refusedAt": "2026-11-19T09:10:00.000Z",
                }],
            })
        );

        let text = output.to_string();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("Account:") && lines[0].ends_with("work (pimdir)"));
        assert!(lines[2].ends_with("perso, work"), "{text}");
        assert!(
            lines[4].starts_with("imap/INBOX  12/40  1 queued  INBOX"),
            "{text}"
        );
        assert!(
            lines[6].starts_with(&format!("{STORE}.1.3  parked")),
            "{text}"
        );
        assert!(lines[6].ends_with("conflict"), "{text}");
        assert!(lines[8].starts_with("9  refused  imap"), "{text}");
        assert!(text.ends_with("NO [CANNOT] refused\n"), "{text}");
    }
}
