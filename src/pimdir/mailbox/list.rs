//! # pimdir mailbox list
//!
//! The `pimdir mailbox list` command, listing the mailboxes of the store
//! with what it holds of each: its counts and its coverage.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_pimdir::{client::reader::PimdirRoundState, collection::PimdirCoverage};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    email::mailbox::{Mailbox, MailboxRole},
    listing::{self, Listing},
    pimdir::{backend::PimdirCoveredMailbox, client::PimdirClient},
};

/// List the mailboxes of the store, with their coverage.
///
/// A sync engine may list a mailbox within a scope, its mail since a date
/// for instance, so a mailbox can hold only part of what its server holds.
/// The coverage says which part: the scope of the last round each source
/// closed, the narrowest of them, and when. A mailbox no round closed on
/// yet has none, and a round under way shows with the scope it lists.
#[derive(Debug, Parser)]
pub struct PimdirMailboxListCommand;

impl PimdirMailboxListCommand {
    /// Lists the mailboxes with their coverage and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let covered = client.list_covered_mailboxes()?;
        let (mut mailboxes, rest): (Vec<_>, Vec<_>) = covered
            .into_iter()
            .map(|covered| (covered.mailbox, (covered.coverage, covered.round)))
            .unzip();
        account.apply_role_aliases(&mut mailboxes, |_| None);

        printer.out(PimdirMailboxListOutput {
            account: account.name().to_string(),
            mailboxes: mailboxes
                .into_iter()
                .zip(rest)
                .map(|(mailbox, (coverage, round))| {
                    PimdirMailbox::from(PimdirCoveredMailbox {
                        mailbox,
                        coverage,
                        round,
                    })
                })
                .collect(),
        })
    }
}

/// A scope's bounds, RFC 3339 instants, `None` for an open end.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailboxCoverage {
    /// The floor, inclusive; `null` for none.
    pub since: Option<String>,
    /// The ceiling, exclusive; `null` for none.
    pub until: Option<String>,
    /// When the round that left it closed.
    pub at: String,
}

/// A round under way: the scope it lists and when it opened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailboxRound {
    /// The floor, inclusive; `null` for none.
    pub since: Option<String>,
    /// The ceiling, exclusive; `null` for none.
    pub until: Option<String>,
    /// When it opened.
    pub started_at: String,
}

/// One mailbox of the store.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailbox {
    /// The collection id, which the other commands take as the mailbox.
    pub id: String,
    /// The collection's name.
    pub name: String,
    /// The special-use role, `null` when none is stated.
    #[schemars(with = "Option<String>")]
    pub role: Option<MailboxRole>,
    /// The messages it holds.
    pub total: Option<u64>,
    /// Those lacking `\Seen`.
    pub unread: Option<u64>,
    /// The scope it is listed with, `null` until every source closed a
    /// round on it.
    pub coverage: Option<PimdirMailboxCoverage>,
    /// The round under way, `null` when none is.
    pub round: Option<PimdirMailboxRound>,
}

impl From<PimdirCoveredMailbox> for PimdirMailbox {
    fn from(covered: PimdirCoveredMailbox) -> Self {
        let Mailbox {
            id,
            name,
            role,
            total,
            unread,
        } = covered.mailbox;
        Self {
            id,
            name,
            role,
            total,
            unread,
            coverage: covered
                .coverage
                .map(|PimdirCoverage { scope, at }| PimdirMailboxCoverage {
                    since: scope.since,
                    until: scope.until,
                    at,
                }),
            round: covered.round.map(
                |PimdirRoundState { scope, started_at }| PimdirMailboxRound {
                    since: scope.since,
                    until: scope.until,
                    started_at,
                },
            ),
        }
    }
}

/// The `pimdir mailbox list` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMailboxListOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The mailboxes, sorted by id.
    pub mailboxes: Vec<PimdirMailbox>,
}

impl fmt::Display for PimdirMailboxListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Pimdir));

        for mailbox in &self.mailboxes {
            let role = mailbox.role.as_ref().map(ToString::to_string);
            let covered = mailbox.coverage.as_ref();
            let covered = covered.map(|c| span(c.since.as_deref(), c.until.as_deref()));
            let syncing = mailbox.round.as_ref();
            let syncing = syncing.map(|r| span(r.since.as_deref(), r.until.as_deref()));

            listing.items.push([
                sanitize::text(&mailbox.id).into_owned(),
                role.unwrap_or_default(),
                listing::counts(mailbox.unread, mailbox.total),
                covered.unwrap_or_default(),
                syncing.unwrap_or_default(),
                sanitize::text(&mailbox.name).into_owned(),
            ]);
        }

        write!(f, "{listing}")
    }
}

/// A scope as its dates read: `since 2026-09-07`, `2026-01-01 to
/// 2026-09-07`, `all` when it has no bound.
pub(crate) fn span(since: Option<&str>, until: Option<&str>) -> String {
    let day = |instant: &str| sanitize::text(instant.get(..10).unwrap_or(instant)).into_owned();
    match (since.map(day), until.map(day)) {
        (None, None) => "all".into(),
        (Some(since), None) => format!("since {since}"),
        (None, Some(until)) => format!("before {until}"),
        (Some(since), Some(until)) => format!("{since} to {until}"),
    }
}

#[cfg(test)]
mod tests {
    use io_pimdir::collection::PimdirScope;

    use crate::pimdir::mailbox::list::*;

    #[test]
    fn a_scope_reads_as_its_days() {
        assert_eq!(span(None, None), "all");
        assert_eq!(span(Some("2026-09-07T00:00:00Z"), None), "since 2026-09-07");
        assert_eq!(
            span(None, Some("2026-09-07T00:00:00Z")),
            "before 2026-09-07"
        );
        assert_eq!(
            span(Some("2026-01-01T00:00:00Z"), Some("2026-09-07T00:00:00Z")),
            "2026-01-01 to 2026-09-07"
        );
    }

    #[test]
    fn coverage_and_round_render_as_camel_case_json() {
        let mailbox = PimdirMailbox::from(PimdirCoveredMailbox {
            mailbox: Mailbox {
                id: "imap/INBOX".into(),
                name: "INBOX".into(),
                role: Some(MailboxRole::Inbox),
                total: Some(12),
                unread: Some(3),
            },
            coverage: Some(PimdirCoverage {
                scope: PimdirScope::since("2026-09-07T00:00:00Z"),
                at: "2026-10-07T10:00:00Z".into(),
            }),
            round: Some(PimdirRoundState {
                scope: PimdirScope::unbounded(),
                started_at: "2026-10-07T10:05:00Z".into(),
            }),
        });

        assert_eq!(
            serde_json::to_value(&mailbox).unwrap(),
            serde_json::json!({
                "id": "imap/INBOX",
                "name": "INBOX",
                "role": "inbox",
                "total": 12,
                "unread": 3,
                "coverage": {
                    "since": "2026-09-07T00:00:00Z",
                    "until": null,
                    "at": "2026-10-07T10:00:00Z",
                },
                "round": {
                    "since": null,
                    "until": null,
                    "startedAt": "2026-10-07T10:05:00Z",
                },
            })
        );
    }

    #[test]
    fn a_mailbox_no_round_closed_on_has_null_coverage() {
        let mailbox = PimdirMailbox::from(PimdirCoveredMailbox {
            mailbox: Mailbox {
                id: "imap/Work".into(),
                name: "Work".into(),
                role: None,
                total: Some(0),
                unread: Some(0),
            },
            coverage: None,
            round: None,
        });
        let json = serde_json::to_value(&mailbox).unwrap();
        assert_eq!(json["coverage"], serde_json::Value::Null);
        assert_eq!(json["round"], serde_json::Value::Null);
        assert_eq!(json["role"], serde_json::Value::Null);
    }
}
