// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{convert::Infallible, fmt, str::FromStr};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use comfy_table::{Cell, Row, Table};
use io_jmap::rfc8621::mailbox::{
    Mailbox, MailboxFilter, MailboxRole, MailboxSortComparator, MailboxSortProperty,
};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::jmap::client::JmapClient;

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
    pub fn execute(self, printer: &mut impl Printer, mut client: JmapClient) -> Result<()> {
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

        let output = client.mailbox_query(
            filter,
            sort,
            Some(self.page.saturating_sub(1) * self.page_size),
            Some(self.page_size),
            None,
        )?;

        let table = MailboxesTable {
            preset: client.account.table_preset().to_string(),
            mailboxes: output.mailboxes,
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
