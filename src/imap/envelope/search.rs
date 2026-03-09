use std::fmt;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_imap::{
    coroutines::{search::*, select::*},
    types::{
        core::{AString, Vec1},
        datetime::NaiveDate,
        search::SearchKey,
    },
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxSelectFlag},
    stream,
};

/// Search IMAP messages by criteria.
///
/// This command searches for messages matching the given criteria and
/// returns a list of matching sequence numbers or UIDs.
///
/// Query syntax (multiple terms are ANDed together):
///   - from:alice     - messages from "alice"
///   - to:bob         - messages to "bob"
///   - cc:charlie     - messages CC'd to "charlie"
///   - bcc:dave       - messages BCC'd to "dave"
///   - subject:hello  - messages with "hello" in subject
///   - body:keyword   - messages with "keyword" in body
///   - text:keyword   - messages with "keyword" in header or body
///   - seen           - messages that have been read
///   - unseen         - messages that have not been read
///   - flagged        - messages that are flagged
///   - answered       - messages that have been answered
///   - deleted        - messages marked for deletion
///   - draft          - draft messages
///   - before:2024-01-15 - messages before date
///   - since:2024-01-01  - messages since date
///   - on:2024-01-10     - messages on date
///   - larger:1000       - messages larger than 1000 bytes
///   - smaller:5000      - messages smaller than 5000 bytes
///   - all               - all messages
#[derive(Debug, Parser)]
pub struct SearchEnvelopesCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub select: MailboxSelectFlag,

    /// Search query (e.g., "from:alice unseen").
    #[arg(name = "query", value_name = "QUERY", default_value = "all")]
    pub query: String,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl SearchEnvelopesCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (mut context, mut stream) = stream::connect(account.backend)?;

        let mailbox = self.mailbox.name.try_into()?;

        if self.select.r#true {
            let mut arg = None;
            let mut coroutine = ImapSelect::new(context, mailbox);

            context = loop {
                match coroutine.resume(arg.take()) {
                    ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                    ImapSelectResult::Ok { context, .. } => break context,
                    ImapSelectResult::Err { err, .. } => bail!(err),
                }
            };
        }

        let criteria = parse_query(&self.query)?;

        let mut arg = None;
        let mut coroutine = ImapSearch::new(context, criteria, !self.seq);

        let ids = loop {
            match coroutine.resume(arg.take()) {
                ImapSearchResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapSearchResult::Ok { ids, .. } => break ids,
                ImapSearchResult::Err { err, .. } => bail!(err),
            }
        };

        let table = SearchTable {
            preset: account.table_preset,
            arrangement: account.table_arrangement,
            ids: ids
                .into_iter()
                .map(|id| SearchResult { id: id.get() })
                .collect(),
            uid_mode: !self.seq,
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchResult {
    pub id: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
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
            table.add_row(Row::from([Cell::new(result.id)]));
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        writeln!(f, "Found {} message(s)", self.ids.len())?;
        Ok(())
    }
}

/// Parse a query string into search criteria.
///
/// Multiple terms are ANDed together.
pub fn parse_query(query: &str) -> Result<Vec1<SearchKey<'static>>> {
    let mut keys: Vec<SearchKey<'static>> = Vec::new();

    for term in query.split_whitespace() {
        let key = parse_term(term)?;
        keys.push(key);
    }

    if keys.is_empty() {
        keys.push(SearchKey::All);
    }

    Ok(Vec1::unvalidated(keys))
}

fn parse_term(term: &str) -> Result<SearchKey<'static>> {
    let term_lower = term.to_lowercase();

    // Simple flag keywords
    match term_lower.as_str() {
        "all" => return Ok(SearchKey::All),
        "seen" => return Ok(SearchKey::Seen),
        "unseen" => return Ok(SearchKey::Unseen),
        "flagged" => return Ok(SearchKey::Flagged),
        "unflagged" => return Ok(SearchKey::Unflagged),
        "answered" => return Ok(SearchKey::Answered),
        "unanswered" => return Ok(SearchKey::Unanswered),
        "deleted" => return Ok(SearchKey::Deleted),
        "undeleted" => return Ok(SearchKey::Undeleted),
        "draft" => return Ok(SearchKey::Draft),
        "undraft" => return Ok(SearchKey::Undraft),
        "new" => return Ok(SearchKey::New),
        "old" => return Ok(SearchKey::Old),
        "recent" => return Ok(SearchKey::Recent),
        _ => {}
    }

    // Key:value patterns
    if let Some((key, value)) = term.split_once(':') {
        let key_lower = key.to_lowercase();
        let value_str = value.to_string();

        match key_lower.as_str() {
            "from" => {
                let astring = AString::try_from(value_str)?;
                return Ok(SearchKey::From(astring));
            }
            "to" => {
                let astring = AString::try_from(value_str)?;
                return Ok(SearchKey::To(astring));
            }
            "cc" => {
                let astring = AString::try_from(value_str)?;
                return Ok(SearchKey::Cc(astring));
            }
            "bcc" => {
                let astring = AString::try_from(value_str)?;
                return Ok(SearchKey::Bcc(astring));
            }
            "subject" => {
                let astring = AString::try_from(value_str)?;
                return Ok(SearchKey::Subject(astring));
            }
            "body" => {
                let astring = AString::try_from(value_str)?;
                return Ok(SearchKey::Body(astring));
            }
            "text" => {
                let astring = AString::try_from(value_str)?;
                return Ok(SearchKey::Text(astring));
            }
            "before" => {
                let date = parse_date(value)?;
                return Ok(SearchKey::Before(date));
            }
            "since" => {
                let date = parse_date(value)?;
                return Ok(SearchKey::Since(date));
            }
            "on" => {
                let date = parse_date(value)?;
                return Ok(SearchKey::On(date));
            }
            "larger" => {
                let size: u32 = value.parse()?;
                return Ok(SearchKey::Larger(size));
            }
            "smaller" => {
                let size: u32 = value.parse()?;
                return Ok(SearchKey::Smaller(size));
            }
            _ => {}
        }
    }

    bail!("Unknown search term: {term}")
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    // Parse YYYY-MM-DD format
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        bail!("Invalid date format '{s}'. Expected YYYY-MM-DD");
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| anyhow!("Invalid year in date '{s}'"))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| anyhow!("Invalid month in date '{s}'"))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| anyhow!("Invalid day in date '{s}'"))?;

    // Create chrono::NaiveDate first
    let chrono_date = chrono::NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| anyhow!("Invalid date '{s}'"))?;

    // Convert to imap-types NaiveDate
    NaiveDate::try_from(chrono_date).map_err(|e| anyhow!("Invalid date '{s}': {e}"))
}
