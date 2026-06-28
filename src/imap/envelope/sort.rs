use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_imap::{
    rfc3501::select::ImapMailboxSelectOptions,
    rfc5256::sort::ImapMessageSortOptions,
    types::{
        core::Vec1,
        extensions::sort::{SortCriterion, SortKey},
    },
};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::imap::{
    client::ImapClient,
    envelope::search::SearchCriteriaArgs,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
};

/// Sort IMAP messages (SORT, RFC 5256).
///
/// Searches with the given criteria, then returns the matching UIDs
/// (or sequence numbers with --seq) sorted by --sort. Requires the
/// SORT extension.
#[derive(Debug, Parser)]
pub struct ImapEnvelopeSortCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

    /// Sort key.
    #[arg(short = 'S', long, value_name = "KEY", default_value = "date")]
    pub sort: SortKeyArg,

    /// Reverse sort order.
    #[arg(short, long)]
    pub reverse: bool,

    #[command(flatten)]
    pub criteria: SearchCriteriaArgs,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapEnvelopeSortCommand {
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

        let sort_criteria = Vec1::unvalidated(vec![SortCriterion {
            reverse: self.reverse,
            key: self.sort.into(),
        }]);
        let search_criteria = self.criteria.into_criteria()?;

        let fallback = client.sort_fallback();
        let ids = client.sort(
            sort_criteria,
            search_criteria,
            ImapMessageSortOptions {
                uid: !self.seq,
                fallback,
            },
        )?;

        let table = SortResultsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            id_color: account.envelopes_list_table_id_color(),
            uid_mode: !self.seq,
            ids: ids.into_iter().map(|id| id.get()).collect(),
        };

        printer.out(table)
    }
}

/// IMAP SORT key (RFC 5256).
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum SortKeyArg {
    #[default]
    Date,
    Arrival,
    From,
    To,
    Cc,
    Subject,
    Size,
}

impl From<SortKeyArg> for SortKey {
    fn from(arg: SortKeyArg) -> Self {
        match arg {
            SortKeyArg::Date => SortKey::Date,
            SortKeyArg::Arrival => SortKey::Arrival,
            SortKeyArg::From => SortKey::From,
            SortKeyArg::To => SortKey::To,
            SortKeyArg::Cc => SortKey::Cc,
            SortKeyArg::Subject => SortKey::Subject,
            SortKeyArg::Size => SortKey::Size,
        }
    }
}

/// Renderable table of SORT result message ids.
#[derive(Clone, Debug, Serialize)]
pub struct SortResultsTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    #[serde(skip)]
    id_color: Color,
    uid_mode: bool,
    ids: Vec<u32>,
}

impl fmt::Display for SortResultsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        let id_header = if self.uid_mode { "UID" } else { "SEQ" };

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new(id_header)]));

        for id in &self.ids {
            table.add_row(Row::from([Cell::new(id).fg(self.id_color)]));
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        writeln!(f, "Found {} message(s)", self.ids.len())
    }
}
