//! # pimdir search
//!
//! The `pimdir search` command, listing the account's messages a query
//! matches through the store's search index.

use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use io_pimdir::client::search::{PimdirSearchOptions, PimdirSearchSort};
use pimalaya_cli::{
    fields::Fields,
    printer::Printer,
    records::{Record, Records},
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    pimdir::{
        client::PimdirClient,
        index::{
            PimdirListStyle, PimdirMailCoverage, PimdirMailHit, PimdirMailThread, local_clock,
            parse_query,
        },
    },
    shared::envelope::list::format_addresses,
};

/// Search the account's messages with the pimdir query language.
///
/// The query is pimdir SEARCH.md §8: words and "phrases" (a trailing `*`
/// for a prefix), `field:value` terms (`from:`, `to:`, `subject:`,
/// `body:`, `is:unread`, `has:attachment`, `date:2026-01..2026-03`,
/// `folder:`, `thread:` and the others), `and`, `or`, `not` or a leading
/// `-`, and parentheses. It is read over the account's mail, its dates
/// in the local zone (`TZ`, then the system's).
///
/// The answer comes from the store's search index, which the sync engine
/// builds and refreshes after each run: a message is found once it is
/// synced and indexed, and a queued write shows in no answer. A line
/// under the list says what the answer could not see: messages whose
/// body is not local, mail outside what the store holds, the index lagging
/// the store.
#[derive(Debug, Parser)]
pub struct PimdirSearchCommand {
    /// Page number, starting at 1.
    #[arg(long, short = 'p')]
    #[arg(value_name = "N", default_value = "1")]
    pub page: usize,
    /// Maximum number of messages per page, threads with `--threads`.
    ///
    /// Omitted, the configured `envelope.list.page-size` answers, and 25
    /// is the hard fallback.
    #[arg(long = "page-size", short = 's')]
    #[arg(value_name = "N")]
    pub page_size: Option<usize>,
    /// The order of the messages.
    #[arg(long, value_enum, default_value_t)]
    pub sort: PimdirSearchSortArg,
    /// Collapse the messages to their threads.
    ///
    /// One row per thread, with how many messages it holds in the account,
    /// its last date and the ids of the messages the query matched; its id
    /// is what `thread:` takes.
    #[arg(long)]
    pub threads: bool,
    /// The query, its words joined by spaces.
    #[arg(value_name = "QUERY", required = true)]
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub query: Vec<String>,
}

impl PimdirSearchCommand {
    /// Searches the index and prints one page of messages or threads.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let query = parse_query(&self.query, printer.is_json())?;
        let limit = self
            .page_size
            .unwrap_or(account.envelopes_list_page_size() as usize)
            .max(1);
        let options = PimdirSearchOptions {
            sort: self.sort.into(),
            limit,
            offset: self.page.saturating_sub(1).saturating_mul(limit),
            clock: local_clock(),
        };
        if self.threads {
            let page = client.search_mail_threads(query, &options)?;
            return printer.out(PimdirSearchThreadsOutput {
                threads: page.threads,
                more: page.more,
                coverage: page.coverage,
            });
        }

        let page = client.search_mail(query, &options)?;
        printer.out(PimdirSearchOutput {
            style: PimdirListStyle::new(account),
            hits: page.hits,
            more: page.more,
            coverage: page.coverage,
        })
    }
}

/// The order of a search (pimdir SEARCH §8.2).
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum PimdirSearchSortArg {
    /// Newest first.
    #[default]
    Date,
    /// By subject, A to Z.
    Name,
    /// Best match first, the messages no word ranks by date after.
    Relevance,
}

impl From<PimdirSearchSortArg> for PimdirSearchSort {
    fn from(sort: PimdirSearchSortArg) -> Self {
        match sort {
            PimdirSearchSortArg::Date => Self::Date,
            PimdirSearchSortArg::Name => Self::Name,
            PimdirSearchSortArg::Relevance => Self::Relevance,
        }
    }
}

/// The `pimdir search` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirSearchOutput {
    /// How the list renders.
    #[serde(skip)]
    pub style: PimdirListStyle,
    /// The messages of this page.
    pub hits: Vec<PimdirMailHit>,
    /// Whether messages follow this page.
    pub more: bool,
    /// What the answer could not see.
    pub coverage: PimdirMailCoverage,
}

impl fmt::Display for PimdirSearchOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hits = self.style.hits(&self.hits);
        self.coverage.write_under(f, &hits)
    }
}

/// The `pimdir search --threads` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirSearchThreadsOutput {
    /// The threads of this page, in the order of their first message.
    pub threads: Vec<PimdirMailThread>,
    /// Whether threads follow this page.
    pub more: bool,
    /// What the answer could not see.
    pub coverage: PimdirMailCoverage,
}

impl fmt::Display for PimdirSearchThreadsOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut records = Records::default();

        for thread in &self.threads {
            let first = thread.hits.first();
            let last = thread
                .last
                .as_deref()
                .map(|last| last.get(..10).unwrap_or(last))
                .unwrap_or_default();
            let from = first
                .map(|hit| format_addresses(&hit.envelope.from))
                .unwrap_or_default();
            let subject = first
                .map(|hit| hit.envelope.subject.as_str())
                .unwrap_or_default();
            let ids: Vec<&str> = thread
                .hits
                .iter()
                .map(|hit| hit.envelope.id.as_str())
                .collect();

            let mut fields = Fields::default();
            fields.push("Messages", thread.count);
            fields.push("Last", sanitize::text(last));
            fields.push("Hits", sanitize::text(&ids.join(", ")));
            fields.push("From", sanitize::text(&from));

            let title = thread.thread.as_deref().unwrap_or("-");

            records.push(Record {
                title: sanitize::text(title).into_owned(),
                state: Vec::new(),
                fields,
                body: sanitize::text(subject).into_owned(),
            });
        }

        self.coverage.write_under(f, &records)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        email::{address::Address, envelope::Envelope},
        pimdir::{
            index::{PimdirListStyle, PimdirMailCoverage, PimdirMailHit, PimdirMailMatched},
            search::PimdirSearchOutput,
        },
    };

    /// A hit prints as a record, its subject then its snippet as the body,
    /// and what the answer could not see a blank line under the records.
    #[test]
    fn hits_print_as_records_above_the_coverage() {
        let envelope = Envelope {
            id: String::from("7"),
            message_id: None,
            in_reply_to: Vec::new(),
            flags: Default::default(),
            subject: String::from("Quarterly report"),
            from: vec![Address {
                name: Some(String::from("Alice")),
                email: String::from("alice@example.org"),
            }],
            to: Vec::new(),
            date: None,
            size: 0,
            has_attachment: Some(true),
        };
        let output = PimdirSearchOutput {
            style: PimdirListStyle {
                datetime_fmt: String::from("%F"),
                datetime_local_tz: false,
            },
            hits: vec![PimdirMailHit {
                envelope,
                mailboxes: vec![String::from("imap/INBOX"), String::from("imap/Archive")],
                matched: Some(PimdirMailMatched::Body),
                snippet: Some(String::from("the  figures\nare in")),
            }],
            more: false,
            coverage: PimdirMailCoverage {
                bodiless: 1,
                ..Default::default()
            },
        };

        let expected = "\
7 (unread, attachment)
From:     Alice <alice@example.org>
Date:
Mailbox:  imap/INBOX, imap/Archive
Matched:  body

    Quarterly report

    the figures are in

Incomplete: 1 message(s) have no local body and matched on their headers alone
";
        assert_eq!(output.to_string(), expected);
    }
}
