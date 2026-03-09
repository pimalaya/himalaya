use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{presets, Cell, ContentArrangement, Row, Table};
use io_imap::{
    coroutines::{select::*, sort::*},
    types::{
        core::Vec1,
        extensions::sort::{SortCriterion, SortKey},
    },
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::{Serialize, Serializer};

use crate::{
    config::ImapConfig,
    imap::{envelope::search::parse_query, mailbox::arg::MailboxNameOptionalArg, stream},
};

/// Sort messages by criteria.
///
/// This command searches for messages matching the given query and
/// returns them sorted by the specified criteria. Requires the SORT
/// IMAP extension.
///
/// Sort criteria:
///   - date      - sort by Date header
///   - arrival   - sort by internal date (arrival time)
///   - from      - sort by From header
///   - to        - sort by To header
///   - cc        - sort by Cc header
///   - subject   - sort by Subject header
///   - size      - sort by message size
#[derive(Debug, Parser)]
pub struct SortEnvelopesCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalArg,

    /// Sort criteria (e.g., "date", "from", "subject", "size").
    #[arg(short = 'S', long, default_value = "date")]
    pub sort: String,

    /// Reverse sort order.
    #[arg(short, long)]
    pub reverse: bool,

    /// Search query (same syntax as search command).
    #[arg(name = "query", value_name = "QUERY", default_value = "all")]
    pub query: String,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl SortEnvelopesCommand {
    pub fn exec(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        let (context, mut stream) = stream::connect(config)?;

        let mailbox = self.mailbox.name.try_into()?;

        // SELECT mailbox
        let mut arg = None;
        let mut coroutine = ImapSelect::new(context, mailbox);

        let context = loop {
            match coroutine.resume(arg.take()) {
                ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapSelectResult::Ok { context, .. } => break context,
                ImapSelectResult::Err { err, .. } => bail!(err),
            }
        };

        // Parse sort criteria
        let sort_key = parse_sort_key(&self.sort)?;
        let sort_criteria = Vec1::unvalidated(vec![SortCriterion {
            reverse: self.reverse,
            key: sort_key,
        }]);

        // Parse search criteria
        let search_criteria = parse_query(&self.query)?;

        // SORT
        let mut arg = None;
        let mut coroutine = ImapSort::new(context, sort_criteria, search_criteria, !self.seq);

        let ids = loop {
            match coroutine.resume(arg.take()) {
                ImapSortResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapSortResult::Ok { ids, .. } => break ids,
                ImapSortResult::Err { err, .. } => bail!(err),
            }
        };

        let table = SortResultsTable::new(ids, !self.seq);

        printer.out(table)?;
        Ok(())
    }
}

fn parse_sort_key(s: &str) -> Result<SortKey> {
    match s.to_lowercase().as_str() {
        "date" => Ok(SortKey::Date),
        "arrival" => Ok(SortKey::Arrival),
        "from" => Ok(SortKey::From),
        "to" => Ok(SortKey::To),
        "cc" => Ok(SortKey::Cc),
        "subject" => Ok(SortKey::Subject),
        "size" => Ok(SortKey::Size),
        _ => bail!(
            "Unknown sort key: {s}. Valid options: date, arrival, from, to, cc, subject, size"
        ),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SortResult {
    pub id: u32,
}

pub struct SortResultsTable {
    results: Vec<SortResult>,
    uid_mode: bool,
}

impl SortResultsTable {
    pub fn new(ids: Vec<std::num::NonZeroU32>, uid_mode: bool) -> Self {
        let results = ids
            .into_iter()
            .map(|id| SortResult { id: id.get() })
            .collect();
        Self { results, uid_mode }
    }
}

impl fmt::Display for SortResultsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        let id_header = if self.uid_mode { "UID" } else { "SEQ" };

        table
            .load_preset(presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([Cell::new(id_header)]));

        for result in &self.results {
            table.add_row(Row::from([Cell::new(result.id)]));
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        writeln!(f, "Found {} message(s)", self.results.len())?;
        Ok(())
    }
}

impl Serialize for SortResultsTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.results.serialize(serializer)
    }
}
