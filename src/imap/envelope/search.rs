use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_imap::{
    rfc3501::{search::ImapMessageSearchOptions, select::ImapMailboxSelectOptions},
    types::{
        core::{AString, Vec1},
        datetime::NaiveDate,
        search::SearchKey,
    },
};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
};

/// Search IMAP messages (SEARCH, RFC 3501).
///
/// Returns the UIDs (or sequence numbers with --seq) of messages
/// matching the given criteria. Each criteria flag maps to one IMAP
/// search key and multiple flags are ANDed; with no criteria, every
/// message matches.
#[derive(Debug, Parser)]
pub struct ImapEnvelopeSearchCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

    #[command(flatten)]
    pub criteria: SearchCriteriaArgs,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapEnvelopeSearchCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            client.select(mailbox, ImapMailboxSelectOptions::default())?;
        }

        let criteria = self.criteria.into_criteria()?;
        let ids = client.search(criteria, ImapMessageSearchOptions { uid: !self.seq })?;

        let table = SearchTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            id_color: account.envelopes_list_table_id_color(),
            ids: ids
                .into_iter()
                .map(|id| SearchResult { id: id.get() })
                .collect(),
            uid_mode: !self.seq,
        };

        printer.out(table)
    }
}

/// IMAP SEARCH criteria (RFC 3501).
///
/// Each flag maps to one IMAP search key; multiple flags are ANDed
/// together. With no flag set, the criteria resolve to `ALL`. Search
/// keys not exposed here (OR, NOT, HEADER, ...) are reachable through
/// the raw passthrough.
#[derive(Debug, Parser)]
pub struct SearchCriteriaArgs {
    /// Match messages whose From header contains TEXT.
    #[arg(long, value_name = "TEXT")]
    pub from: Option<String>,
    /// Match messages whose To header contains TEXT.
    #[arg(long, value_name = "TEXT")]
    pub to: Option<String>,
    /// Match messages whose Cc header contains TEXT.
    #[arg(long, value_name = "TEXT")]
    pub cc: Option<String>,
    /// Match messages whose Bcc header contains TEXT.
    #[arg(long, value_name = "TEXT")]
    pub bcc: Option<String>,
    /// Match messages whose Subject header contains TEXT.
    #[arg(long, value_name = "TEXT")]
    pub subject: Option<String>,
    /// Match messages whose body contains TEXT.
    #[arg(long, value_name = "TEXT")]
    pub body: Option<String>,
    /// Match messages whose headers or body contain TEXT.
    #[arg(long, value_name = "TEXT")]
    pub text: Option<String>,

    /// Match messages received before DATE (YYYY-MM-DD).
    #[arg(long, value_name = "DATE", value_parser = date_parser)]
    pub before: Option<NaiveDate>,
    /// Match messages received since DATE (YYYY-MM-DD).
    #[arg(long, value_name = "DATE", value_parser = date_parser)]
    pub since: Option<NaiveDate>,
    /// Match messages received on DATE (YYYY-MM-DD).
    #[arg(long, value_name = "DATE", value_parser = date_parser)]
    pub on: Option<NaiveDate>,

    /// Match messages larger than BYTES.
    #[arg(long, value_name = "BYTES")]
    pub larger: Option<u32>,
    /// Match messages smaller than BYTES.
    #[arg(long, value_name = "BYTES")]
    pub smaller: Option<u32>,

    /// Match \Seen messages.
    #[arg(long)]
    pub seen: bool,
    /// Match messages without the \Seen flag.
    #[arg(long)]
    pub unseen: bool,
    /// Match \Flagged messages.
    #[arg(long)]
    pub flagged: bool,
    /// Match messages without the \Flagged flag.
    #[arg(long)]
    pub unflagged: bool,
    /// Match \Answered messages.
    #[arg(long)]
    pub answered: bool,
    /// Match messages without the \Answered flag.
    #[arg(long)]
    pub unanswered: bool,
    /// Match \Deleted messages.
    #[arg(long)]
    pub deleted: bool,
    /// Match messages without the \Deleted flag.
    #[arg(long)]
    pub undeleted: bool,
    /// Match \Draft messages.
    #[arg(long)]
    pub draft: bool,
    /// Match messages without the \Draft flag.
    #[arg(long)]
    pub undraft: bool,
    /// Match \Recent messages that are also unseen (NEW).
    #[arg(long)]
    pub new: bool,
    /// Match messages without the \Recent flag (OLD).
    #[arg(long)]
    pub old: bool,
    /// Match \Recent messages.
    #[arg(long)]
    pub recent: bool,
}

impl SearchCriteriaArgs {
    /// Folds every set flag into an IMAP search key, ANDed together;
    /// resolves to a single `ALL` key when nothing is set.
    pub fn into_criteria(self) -> Result<Vec1<SearchKey<'static>>> {
        let mut keys: Vec<SearchKey<'static>> = Vec::new();

        if let Some(text) = self.from {
            keys.push(SearchKey::From(AString::try_from(text)?));
        }
        if let Some(text) = self.to {
            keys.push(SearchKey::To(AString::try_from(text)?));
        }
        if let Some(text) = self.cc {
            keys.push(SearchKey::Cc(AString::try_from(text)?));
        }
        if let Some(text) = self.bcc {
            keys.push(SearchKey::Bcc(AString::try_from(text)?));
        }
        if let Some(text) = self.subject {
            keys.push(SearchKey::Subject(AString::try_from(text)?));
        }
        if let Some(text) = self.body {
            keys.push(SearchKey::Body(AString::try_from(text)?));
        }
        if let Some(text) = self.text {
            keys.push(SearchKey::Text(AString::try_from(text)?));
        }

        if let Some(date) = self.before {
            keys.push(SearchKey::Before(date));
        }
        if let Some(date) = self.since {
            keys.push(SearchKey::Since(date));
        }
        if let Some(date) = self.on {
            keys.push(SearchKey::On(date));
        }

        if let Some(size) = self.larger {
            keys.push(SearchKey::Larger(size));
        }
        if let Some(size) = self.smaller {
            keys.push(SearchKey::Smaller(size));
        }

        if self.seen {
            keys.push(SearchKey::Seen);
        }
        if self.unseen {
            keys.push(SearchKey::Unseen);
        }
        if self.flagged {
            keys.push(SearchKey::Flagged);
        }
        if self.unflagged {
            keys.push(SearchKey::Unflagged);
        }
        if self.answered {
            keys.push(SearchKey::Answered);
        }
        if self.unanswered {
            keys.push(SearchKey::Unanswered);
        }
        if self.deleted {
            keys.push(SearchKey::Deleted);
        }
        if self.undeleted {
            keys.push(SearchKey::Undeleted);
        }
        if self.draft {
            keys.push(SearchKey::Draft);
        }
        if self.undraft {
            keys.push(SearchKey::Undraft);
        }
        if self.new {
            keys.push(SearchKey::New);
        }
        if self.old {
            keys.push(SearchKey::Old);
        }
        if self.recent {
            keys.push(SearchKey::Recent);
        }

        if keys.is_empty() {
            keys.push(SearchKey::All);
        }

        Ok(Vec1::unvalidated(keys))
    }
}

/// Clap value parser for a YYYY-MM-DD search date.
fn date_parser(s: &str) -> Result<NaiveDate, String> {
    let date = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| format!("expected a YYYY-MM-DD date, got `{s}`"))?;

    NaiveDate::try_from(date).map_err(|e| format!("invalid date `{s}`: {e}"))
}

/// One row of the SEARCH results table: a single message id.
#[derive(Clone, Debug, Serialize)]
pub struct SearchResult {
    pub id: u32,
}

/// Renderable table of SEARCH result message ids.
#[derive(Clone, Debug, Serialize)]
pub struct SearchTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    #[serde(skip)]
    id_color: Color,
    uid_mode: bool,
    ids: Vec<SearchResult>,
}

impl fmt::Display for SearchTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        let id_header = if self.uid_mode { "UID" } else { "SEQ" };

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new(id_header)]));

        for result in &self.ids {
            table.add_row(Row::from([Cell::new(result.id).fg(self.id_color)]));
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        writeln!(f, "Found {} message(s)", self.ids.len())?;
        Ok(())
    }
}
