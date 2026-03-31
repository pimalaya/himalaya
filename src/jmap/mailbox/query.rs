use std::{convert::Infallible, fmt, str::FromStr};

use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use comfy_table::{Cell, Row, Table};
use io_jmap::{
    rfc8621::coroutines::mailbox_query::{JmapMailboxQuery, JmapMailboxQueryResult},
    rfc8621::types::mailbox::{
        Mailbox, MailboxFilter, MailboxRole, MailboxSortComparator, MailboxSortProperty,
    },
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::jmap::account::JmapAccount;

/// Query JMAP mailboxes (Mailbox/query + Mailbox/get).
///
/// Lists, filters and sorts mailboxes.
#[derive(Debug, Parser)]
pub struct JmapMailboxQueryCommand {
    /// Filter by parent mailbox identifier.
    #[arg(long, value_name = "ID")]
    pub parent_id: Option<String>,

    /// Filter by role [possible values: inbox, archive, drafts,
    /// flagged, important, junk, sent, subscribed, trash].
    #[arg(long, value_name = "ROLE")]
    pub role: Option<RoleArg>,

    /// Filter by substring name match.
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,

    /// List all mailboxes, not just subscribed ones.
    #[arg(long, default_value_t)]
    pub all: bool,

    /// Only return mailboxes that have a role.
    #[arg(long, default_value_t)]
    pub has_any_role: bool,

    /// Sort by property.
    #[arg(long, value_name = "PROP", default_value_t)]
    pub sort: SortArg,

    /// Sort in descending order.
    #[arg(long, default_value_t)]
    pub desc: bool,

    /// Number of mailboxes to display per page.
    #[arg(long, short = 's', value_name = "N", default_value = "10")]
    pub page_size: u64,

    /// Page index, starting from 1.
    #[arg(long, short, value_name = "N", default_value = "1")]
    pub page: u64,
}

impl JmapMailboxQueryCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let filter = {
            let f = MailboxFilter {
                parent_id: self.parent_id,
                role: self.role.map(Into::into),
                name: self.name,
                is_subscribed: if self.all { None } else { Some(true) },
                has_any_role: if self.has_any_role { Some(true) } else { None },
            };

            let has_one_filter = f.parent_id.is_some()
                || f.role.is_some()
                || f.name.is_some()
                || f.is_subscribed.is_some()
                || f.has_any_role.is_some();

            if has_one_filter {
                Some(f)
            } else {
                None
            }
        };

        let sort = Some(vec![MailboxSortComparator {
            property: self.sort.into(),
            is_ascending: Some(!self.desc),
        }]);

        let mut arg = None;
        let mut coroutine = JmapMailboxQuery::new(
            &jmap.session,
            &jmap.http_auth,
            filter,
            sort,
            Some(self.page.saturating_sub(1) * self.page_size),
            Some(self.page_size),
            None,
        )?;

        let mailboxes = loop {
            match coroutine.resume(arg.take()) {
                JmapMailboxQueryResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapMailboxQueryResult::Ok { mailboxes, .. } => break mailboxes,
                JmapMailboxQueryResult::Err { err, .. } => bail!(err),
            }
        };

        let table = MailboxesTable {
            preset: account.table_preset,
            mailboxes,
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct MailboxesTable {
    #[serde(skip)]
    pub preset: String,
    pub mailboxes: Vec<Mailbox>,
}

impl fmt::Display for MailboxesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("NAME"),
                Cell::new("ROLE"),
                Cell::new("TOTAL"),
                Cell::new("UNREAD"),
                Cell::new("SUBSCRIBED"),
            ]))
            .add_rows(self.mailboxes.iter().map(|r| {
                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(r.id.as_deref().unwrap_or("Unknown")))
                    .add_cell(Cell::new(r.name.as_deref().unwrap_or("Unknown")))
                    .add_cell(match r.role.as_ref() {
                        Some(r) => Cell::new(r.to_string()),
                        None => Cell::new(""),
                    })
                    .add_cell(Cell::new(r.total_emails))
                    .add_cell(Cell::new(r.unread_emails))
                    .add_cell(Cell::new(if r.is_subscribed { "yes" } else { "" }));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

#[derive(Clone, Debug)]
pub enum RoleArg {
    Inbox,
    Archive,
    Drafts,
    Flagged,
    Important,
    Junk,
    Sent,
    Subscribed,
    Trash,
    Other(String),
}

impl FromStr for RoleArg {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inbox" => Ok(Self::Inbox),
            "archive" => Ok(Self::Archive),
            "drafts" => Ok(Self::Drafts),
            "flagged" => Ok(Self::Flagged),
            "important" => Ok(Self::Important),
            "junk" => Ok(Self::Junk),
            "sent" => Ok(Self::Sent),
            "subscribed" => Ok(Self::Subscribed),
            "trash" => Ok(Self::Trash),
            other => Ok(Self::Other(other.to_owned())),
        }
    }
}

impl From<RoleArg> for MailboxRole {
    fn from(arg: RoleArg) -> Self {
        match arg {
            RoleArg::Inbox => MailboxRole::Inbox,
            RoleArg::Archive => MailboxRole::Archive,
            RoleArg::Drafts => MailboxRole::Drafts,
            RoleArg::Flagged => MailboxRole::Flagged,
            RoleArg::Important => MailboxRole::Important,
            RoleArg::Junk => MailboxRole::Junk,
            RoleArg::Sent => MailboxRole::Sent,
            RoleArg::Subscribed => MailboxRole::Subscribed,
            RoleArg::Trash => MailboxRole::Trash,
            RoleArg::Other(s) => MailboxRole::Other(s),
        }
    }
}

#[derive(Clone, Debug, Default, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum SortArg {
    Name,
    #[default]
    SortOrder,
    ParentId,
}

impl fmt::Display for SortArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name => write!(f, "name"),
            Self::SortOrder => write!(f, "sort-order"),
            Self::ParentId => write!(f, "parent-id"),
        }
    }
}

impl From<SortArg> for MailboxSortProperty {
    fn from(arg: SortArg) -> Self {
        match arg {
            SortArg::Name => MailboxSortProperty::Name,
            SortArg::SortOrder => MailboxSortProperty::SortOrder,
            SortArg::ParentId => MailboxSortProperty::ParentId,
        }
    }
}
