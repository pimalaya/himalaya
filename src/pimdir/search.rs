//! # pimdir search
//!
//! The `pimdir search` command, listing the account's messages a query
//! matches through the store's search index.

use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use io_pimdir::client::search::{PimdirSearchCursor, PimdirSearchOptions, PimdirSearchSort};
use pimalaya_cli::{
    listing::{Listing, ListingPage},
    printer::Printer,
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    error::{ItemError, page_token_invalid, partial_when},
    pimdir::{
        client::PimdirClient,
        index::{
            PimdirListStyle, PimdirMailCoverage, PimdirMailHit, PimdirMailMatched,
            PimdirMailThread, local_clock, parse_query,
        },
    },
    unified::{
        backend::Backend,
        envelope::list::{envelope_cells, format_names},
    },
};

/// Search the account's messages with the pimdir query language.
///
/// The query is pimdir SEARCH.md §8: words and "phrases" (a trailing `*`
/// for a prefix), `field:value` terms (`from:`, `to:`, `subject:`,
/// `body:`, `is:unread`, `has:attachment`, `date:2026-01..2026-03`,
/// `folder:`, `thread:` and the others), `and`, `or`, `not` or a leading
/// `-` (its word after `--`), and parentheses. It is read over the
/// account's mail, its dates in the local zone (`TZ`, then the system's).
///
/// The answer comes from the store's search index, which the sync engine
/// builds and refreshes after each run: a message is found once it is
/// synced and indexed, and a queued write shows in no answer. An
/// `Incomplete:` line above the list says what the answer could not see:
/// messages whose body is not local, mail outside what the store holds,
/// the index lagging the store.
///
/// A page ends with the token of the next one, which `--page-token`
/// takes with the same query, order and `--threads`; the last page has
/// none. The next page starts after the last message shown, whatever
/// the index gained or lost since. A message the store cannot read is an
/// item error, the others listed.
#[derive(Debug, Parser)]
pub struct PimdirSearchCommand {
    /// Maximum number of messages per page, threads with `--threads`.
    ///
    /// Omitted, the configured `envelope.list.page-size` answers, and 25
    /// is the hard fallback.
    #[arg(long = "page-size", short = 's')]
    #[arg(value_name = "N")]
    pub page_size: Option<usize>,
    /// Start after the page that printed this token as its `nextPage`.
    #[arg(long = "page-token")]
    #[arg(value_name = "TOKEN")]
    pub page_token: Option<String>,
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
    ///
    /// Options may come before, between or after its words. A word
    /// opening on `-`, such as a negated term, goes after `--`:
    /// `-- -from:alice`, or write `not from:alice`.
    #[arg(value_name = "QUERY", required = true)]
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
        let words = self.query.join(" ");
        let query = parse_query(&self.query, printer.is_json())?;
        let limit = self
            .page_size
            .unwrap_or(account.envelopes_list_page_size() as usize)
            .max(1);
        let place = PimdirSearchPlace {
            account: account.name().to_string(),
            query: words,
            sort: self.sort,
            threads: self.threads,
        };
        let after = self.page_token.as_deref();
        let options = PimdirSearchOptions {
            sort: self.sort.into(),
            limit,
            after: after.map(|token| place.cursor(token)).transpose()?,
            clock: local_clock(),
        };

        if self.threads {
            let page = client.search_mail_threads(query, &options)?;
            let output = PimdirSearchThreadsOutput {
                next_page: page.next.as_ref().map(|next| place.token(next)),
                place,
                threads: page.threads,
                coverage: page.coverage,
                errors: page.errors,
            };

            printer.out(&output)?;
            return partial_when(&output.errors);
        }

        let page = client.search_mail(query, &options)?;
        let output = PimdirSearchOutput {
            next_page: page.next.as_ref().map(|next| place.token(next)),
            place,
            style: PimdirListStyle::new(account),
            hits: page.hits,
            coverage: page.coverage,
            errors: page.errors,
        };

        printer.out(&output)?;
        partial_when(&output.errors)
    }
}

/// The order of a search (pimdir SEARCH §8.2).
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum PimdirSearchSortArg {
    /// Newest first.
    #[default]
    NewestFirst,
    /// Oldest first.
    OldestFirst,
    /// By subject, A to Z.
    Title,
    /// By sender, A to Z.
    Sender,
    /// Best match first, the messages no word ranks by date after.
    Relevance,
}

impl From<PimdirSearchSortArg> for PimdirSearchSort {
    fn from(sort: PimdirSearchSortArg) -> Self {
        match sort {
            PimdirSearchSortArg::NewestFirst => Self::NewestFirst,
            PimdirSearchSortArg::OldestFirst => Self::OldestFirst,
            PimdirSearchSortArg::Title => Self::Title,
            PimdirSearchSortArg::Sender => Self::Sender,
            PimdirSearchSortArg::Relevance => Self::Relevance,
        }
    }
}

/// The `pimdir search` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirSearchOutput {
    /// Where the page stands.
    #[serde(skip)]
    pub place: PimdirSearchPlace,
    /// How the list renders.
    #[serde(skip)]
    pub style: PimdirListStyle,
    /// The messages of this page.
    pub hits: Vec<PimdirMailHit>,
    /// The number of the next page, `null` once the answer is complete,
    /// given back through `--page-token`.
    pub next_page: Option<String>,
    /// What the answer could not see.
    pub coverage: PimdirMailCoverage,
    /// The error of each message the store could not read, in the order
    /// met.
    pub errors: Vec<ItemError>,
}

impl fmt::Display for PimdirSearchOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = self.place.listing();
        let matched = self.hits.iter().any(|hit| hit.matched.is_some());

        for hit in &self.hits {
            let fmt = &self.style.datetime_fmt;
            let local_tz = self.style.datetime_local_tz;
            let [id, flags, date, names, subject] = envelope_cells(&hit.envelope, fmt, local_tz);
            let mailboxes = sanitize::text(&hit.mailboxes.join(",")).into_owned();

            let mut line = vec![id, flags, date, mailboxes];

            if matched {
                line.push(String::from(match hit.matched {
                    Some(PimdirMailMatched::Body) => "body",
                    Some(PimdirMailMatched::Summary) => "headers",
                    None => "",
                }));
            }

            line.extend([names, subject]);
            listing.items.push(line);
        }

        let nouns = ["message", "messages"];
        let next = self.next_page.as_deref();
        self.place.end(&mut listing, self.hits.len(), nouns, next);
        self.coverage.note(&mut listing);

        for error in &self.errors {
            listing.errors.push(error);
        }

        write!(f, "{listing}")
    }
}

/// The `pimdir search --threads` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirSearchThreadsOutput {
    /// Where the page stands.
    #[serde(skip)]
    pub place: PimdirSearchPlace,
    /// The threads of this page, in the order of their first message.
    pub threads: Vec<PimdirMailThread>,
    /// The number of the next page, `null` once the answer is complete,
    /// given back through `--page-token`.
    pub next_page: Option<String>,
    /// What the answer could not see.
    pub coverage: PimdirMailCoverage,
    /// The error of each message the store could not read, in the order
    /// met.
    pub errors: Vec<ItemError>,
}

impl fmt::Display for PimdirSearchThreadsOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = self.place.listing();

        for thread in &self.threads {
            let first = thread.hits.first();
            let last = thread
                .last_date
                .as_deref()
                .map(|last| last.get(..10).unwrap_or(last))
                .unwrap_or_default();
            let names = first
                .map(|hit| format_names(&hit.envelope.from))
                .unwrap_or_default();
            let subject = first
                .and_then(|hit| hit.envelope.subject.as_deref())
                .unwrap_or_default();
            let ids: Vec<&str> = thread
                .hits
                .iter()
                .map(|hit| hit.envelope.id.as_str())
                .collect();

            listing.items.push([
                sanitize::text(thread.thread.as_deref().unwrap_or_default()).into_owned(),
                thread.count.to_string(),
                sanitize::text(last).into_owned(),
                sanitize::text(&ids.join(",")).into_owned(),
                sanitize::text(&names).into_owned(),
                sanitize::text(subject).into_owned(),
            ]);
        }

        let nouns = ["thread", "threads"];
        let next = self.next_page.as_deref();
        self.place
            .end(&mut listing, self.threads.len(), nouns, next);
        self.coverage.note(&mut listing);

        for error in &self.errors {
            listing.errors.push(error);
        }

        write!(f, "{listing}")
    }
}

/// Where a page of `pimdir search` stands: the account, the query, its
/// order and whether it lists threads, which a page token is bound to.
#[derive(Clone, Debug, Default)]
pub struct PimdirSearchPlace {
    /// The name of the account searched.
    pub account: String,
    /// The query, its words joined by spaces.
    pub query: String,
    /// The order of the hits.
    pub sort: PimdirSearchSortArg,
    /// Whether the page lists threads.
    pub threads: bool,
}

impl PimdirSearchPlace {
    /// A listing headed by the account and the query.
    fn listing(&self) -> Listing<'static> {
        let mut listing = Listing::new(&self.account, Some(Backend::Pimdir.as_str()));
        listing.head("Query", &self.query);
        listing
    }

    /// The FNV-1a hash of the account, the query, the order and the
    /// listing, which a page token starts with.
    fn fingerprint(&self) -> String {
        let sort = PimdirSearchSort::from(self.sort).as_str();
        let threads = if self.threads { "threads" } else { "hits" };
        let parts = [self.account.as_str(), &self.query, sort, threads];
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;

        for byte in parts.join("\0").bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }

        format!("{hash:016x}")
    }

    /// The page token of `next`: the fingerprint, a dot, then the cursor.
    fn token(&self, next: &PimdirSearchCursor) -> String {
        format!("{}.{}", self.fingerprint(), next.token())
    }

    /// The cursor a page token names, refused when this place did not
    /// print it.
    fn cursor(&self, token: &str) -> Result<PimdirSearchCursor> {
        token
            .split_once('.')
            .filter(|(fingerprint, _)| *fingerprint == self.fingerprint())
            .and_then(|(_, cursor)| PimdirSearchCursor::from_token(cursor))
            .ok_or_else(|| page_token_invalid(token))
    }

    /// Adds to `listing` the page the `count` items fill, then the flag
    /// reaching the `next` one.
    fn end(&self, listing: &mut Listing, count: usize, nouns: [&str; 2], next: Option<&str>) {
        listing.page(ListingPage::Token, count, nouns, next);
    }
}

#[cfg(test)]
mod tests {
    use io_pimdir::{client::search::PimdirSearchCursor, search::coverage::PimdirSearchUnread};

    use crate::{
        error::{ErrorCode, ItemError},
        pimdir::{
            index::{PimdirListStyle, PimdirMailCoverage, PimdirMailHit, PimdirMailMatched},
            search::{PimdirSearchOutput, PimdirSearchPlace, PimdirSearchSortArg},
        },
        unified::email::{address::Address, envelope::Envelope},
    };

    /// One hit, on the last page of an incomplete answer.
    fn output() -> PimdirSearchOutput {
        let envelope = Envelope {
            id: String::from("7"),
            message_id: None,
            in_reply_to: Vec::new(),
            flags: Default::default(),
            subject: Some(String::from("Quarterly report")),
            from: vec![Address {
                name: Some(String::from("Alice")),
                email: String::from("alice@example.org"),
            }],
            to: Vec::new(),
            date: None,
            size: None,
            has_attachment: Some(true),
        };
        PimdirSearchOutput {
            place: PimdirSearchPlace {
                account: String::from("work"),
                query: String::from("report"),
                sort: PimdirSearchSortArg::NewestFirst,
                threads: false,
            },
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
            next_page: None,
            coverage: PimdirMailCoverage {
                bodiless: 1,
                counted: PimdirSearchUnread {
                    bodiless: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
            errors: Vec::new(),
        }
    }

    /// A hit prints on one line, mailboxes and match before the sender,
    /// under a header whose `Incomplete:` label widens the column.
    #[test]
    fn a_hit_prints_on_one_line() {
        let expected = "\
Account:     work (pimdir)
Query:       report
Page:        1 message, the last
Incomplete:  1 message(s) have no local body and matched on their headers alone

7  *  @  -  imap/INBOX,imap/Archive  body  Alice  Quarterly report
";
        assert_eq!(output().to_string(), expected);
    }

    /// A page pages as every listing does (`nextPage`, no `more`), and an
    /// unreadable message is an `errors` entry printed after the items.
    #[test]
    fn a_page_names_the_next_and_its_errors() {
        let mut output = output();
        let cursor = PimdirSearchCursor::from_token("1.0.-.-..7.").unwrap();
        let next = output.place.token(&cursor);
        output.next_page = Some(next.clone());
        output.errors = vec![ItemError {
            id: Some(String::from("9")),
            code: Some(ErrorCode::MessageUnreadable),
            error: String::from("Read message `9`"),
            sources: vec![String::from("Invalid column type")],
        }];

        let json = serde_json::to_value(&output).unwrap();
        let mut keys: Vec<&str> = json.as_object().unwrap().keys().map(|k| &**k).collect();
        keys.sort();
        assert_eq!(keys, ["coverage", "errors", "hits", "nextPage"]);
        assert_eq!(json["nextPage"], next.as_str());
        assert_eq!(json["errors"][0]["id"], "9");
        assert_eq!(json["errors"][0]["code"], "message-unreadable");

        let expected = format!(
            "\
Account:     work (pimdir)
Query:       report
Page:        1 message
Incomplete:  1 message(s) have no local body and matched on their headers alone

7  *  @  -  imap/INBOX,imap/Archive  body  Alice  Quarterly report

Next page: --page-token {next}

Error:  9  message-unreadable  Read message `9`
"
        );
        assert_eq!(output.to_string(), expected);
    }

    /// A page token of another query, order, listing or account is
    /// refused, as is an offset page number.
    #[test]
    fn a_page_token_is_bound_to_its_query() {
        let place = output().place;
        let cursor = PimdirSearchCursor::from_token("1.0.-.-..7.").unwrap();
        let token = place.token(&cursor);
        assert_eq!(place.cursor(&token).unwrap(), cursor);

        let others = [
            PimdirSearchPlace {
                query: String::from("invoice"),
                ..place.clone()
            },
            PimdirSearchPlace {
                sort: PimdirSearchSortArg::Title,
                ..place.clone()
            },
            PimdirSearchPlace {
                threads: true,
                ..place.clone()
            },
            PimdirSearchPlace {
                account: String::from("home"),
                ..place.clone()
            },
        ];

        for other in others {
            let error = other.cursor(&token).unwrap_err();
            let code = crate::error::code(&error);
            assert_eq!(code, Some(ErrorCode::PageTokenInvalid));
        }

        for token in ["2", "", "x.1.0.-.-..7."] {
            assert!(place.cursor(token).is_err(), "{token}");
        }
    }
}
