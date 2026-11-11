//! # IMAP sort
//!
//! The `imap sort` command, RFC 5256 `SORT`.

use io_imap::client::ImapClient as _;
use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use io_imap::{
    rfc3501::select::ImapMailboxSelectOptions,
    rfc5256::sort::ImapMessageSortOptions,
    types::{
        core::Vec1,
        extensions::sort::{SortCriterion, SortKey},
    },
};
use pimalaya_cli::{columns::Columns, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    imap::{
        client::ImapClient,
        envelope::search::SearchCriteriaArgs,
        mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
        outcome,
    },
};

/// Sort messages (SORT, RFC 5256).
///
/// Searches with the given criteria, then returns the matching UIDs in
/// the requested order. The server has to advertise the SORT extension.
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
    /// Selects the mailbox unless told not to, then sorts it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        let result = (|| -> Result<_> {
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

            Ok(ImapSortOutput {
                uid_mode: !self.seq,
                ids: ids.into_iter().map(|id| id.get()).collect(),
            })
        })();

        outcome::print(printer, result)
    }
}

/// The property a `SORT` sorts on, per RFC 5256.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum SortKeyArg {
    /// The `Date:` header, the sent-at.
    #[default]
    Date,
    /// The internal date, the received-at.
    Arrival,
    /// The first `From:` address.
    From,
    /// The first `To:` address.
    To,
    /// The first `Cc:` address.
    Cc,
    /// The `Subject:` header, its `Re:` prefix stripped.
    Subject,
    /// The size in octets.
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

/// The `imap sort` output, the message ids in sorted order.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImapSortOutput {
    uid_mode: bool,
    ids: Vec<u32>,
}

impl fmt::Display for ImapSortOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::new([if self.uid_mode { "UID" } else { "SEQ" }]);

        for id in &self.ids {
            columns.push([id]);
        }

        write!(f, "{columns}")
    }
}
