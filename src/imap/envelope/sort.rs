use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{presets, Cell, ContentArrangement, Row, Table};
use io_imap::{
    rfc3501::select::*,
    rfc5256::sort::*,
    types::{
        core::Vec1,
        extensions::sort::{SortCriterion, SortKey},
    },
};
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::imap::{
    account::ImapAccount, envelope::search::parse_query, mailbox::arg::MailboxNameOptionalArg,
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
pub struct ImapEnvelopeSortCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalArg,

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

impl ImapEnvelopeSortCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        // SELECT mailbox
        let mut arg = None;
        let mut coroutine = ImapMailboxSelect::new(imap.context, mailbox);

        let context = loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxSelectResult::Io { input } => {
                    arg = Some(handle(&mut imap.stream, input)?)
                }
                ImapMailboxSelectResult::Ok { context, .. } => break context,
                ImapMailboxSelectResult::Err { err, .. } => bail!(err),
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
        let mut coroutine =
            ImapMailboxSort::new(context, sort_criteria, search_criteria, !self.seq);

        let ids = loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxSortResult::Io { input } => arg = Some(handle(&mut imap.stream, input)?),
                ImapMailboxSortResult::Ok { ids, .. } => break ids,
                ImapMailboxSortResult::Err { err, .. } => bail!(err),
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
            "Unknown sort key `{s}`, valid options: date, arrival, from, to, cc, subject, size"
        ),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SortResultsTable {
    ids: Vec<u32>,
    uid_mode: bool,
}

impl SortResultsTable {
    pub fn new(ids: Vec<std::num::NonZeroU32>, uid_mode: bool) -> Self {
        let ids = ids.into_iter().map(|id| id.get()).collect();
        Self { ids, uid_mode }
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

        for id in &self.ids {
            table.add_row(Row::from([Cell::new(id)]));
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        writeln!(f, "Found {} message(s)", self.ids.len())
    }
}
