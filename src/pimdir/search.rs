//! # pimdir search
//!
//! The `pimdir search` command, listing the account's messages a query
//! matches through the store's search index.

use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use io_pimdir::client::search::{PimdirSearchOptions, PimdirSearchSort};
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
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
        write!(f, "{}", self.style.hits(&self.hits))?;
        write!(f, "{}", self.coverage)
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
        // NOTE: the free-text columns read last, the hit ids joining
        // without a blank.
        let header = ["THREAD", "MESSAGES", "LAST", "HITS", "FROM", "SUBJECT"];
        let mut columns = Columns::new(header);

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

            columns.push([
                sanitize::text(thread.thread.as_deref().unwrap_or_default()).into_owned(),
                thread.count.to_string(),
                sanitize::text(last).into_owned(),
                sanitize::text(&ids.join(",")).into_owned(),
                sanitize::text(&from).into_owned(),
                sanitize::text(subject).into_owned(),
            ]);
        }

        write!(f, "{columns}")?;
        write!(f, "{}", self.coverage)
    }
}
