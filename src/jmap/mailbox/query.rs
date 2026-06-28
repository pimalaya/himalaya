use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use comfy_table::{Cell, Color, Row, Table};
use io_jmap::rfc8621::mailbox::{
    JmapMailbox, JmapMailboxFilter, JmapMailboxRole, JmapMailboxSortComparator,
    JmapMailboxSortProperty, query::JmapMailboxQueryOptions,
};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::jmap::client::JmapClient;

/// Query JMAP mailboxes (Mailbox/query + Mailbox/get).
///
/// Lists, filters and sorts mailboxes.
#[derive(Debug, Parser)]
pub struct JmapMailboxQueryCommand {
    /// Filter by parent mailbox identifier.
    #[arg(long, value_name = "ID")]
    pub parent_id: Option<String>,

    /// Filter by a standard role.
    #[arg(long, value_name = "ROLE", conflicts_with = "custom_role")]
    pub role: Option<RoleArg>,

    /// Filter by a custom (non-standard) role.
    #[arg(long, value_name = "ROLE")]
    pub custom_role: Option<String>,

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
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let filter = {
            let f = JmapMailboxFilter {
                parent_id: self.parent_id,
                role: role_from_args(self.role, self.custom_role),
                name: self.name,
                is_subscribed: if self.all { None } else { Some(true) },
                has_any_role: if self.has_any_role { Some(true) } else { None },
            };

            let has_one_filter = f.parent_id.is_some()
                || f.role.is_some()
                || f.name.is_some()
                || f.is_subscribed.is_some()
                || f.has_any_role.is_some();

            if has_one_filter { Some(f) } else { None }
        };

        let sort = Some(vec![JmapMailboxSortComparator {
            property: self.sort.into(),
            is_ascending: Some(!self.desc),
        }]);

        let output = client.mailbox_query(JmapMailboxQueryOptions {
            filter,
            sort,
            position: Some(self.page.saturating_sub(1) * self.page_size),
            limit: Some(self.page_size),
            properties: None,
        })?;

        let table = MailboxesTable {
            preset: account.table_preset().to_string(),
            colors: MailboxColors {
                id: account.mailboxes_list_table_id_color(),
                name: account.mailboxes_list_table_name_color(),
                total: account.mailboxes_list_table_total_color(),
                unread: account.mailboxes_list_table_unread_color(),
            },
            mailboxes: output.mailboxes,
        };

        printer.out(table)
    }
}

/// Per-column colors for the mailboxes table.
#[derive(Clone, Copy, Debug)]
pub struct MailboxColors {
    pub id: Color,
    pub name: Color,
    pub total: Color,
    pub unread: Color,
}

impl Default for MailboxColors {
    fn default() -> Self {
        Self {
            id: Color::Reset,
            name: Color::Reset,
            total: Color::Reset,
            unread: Color::Reset,
        }
    }
}

/// Renderable table of mailboxes.
#[derive(Clone, Debug, Default, Serialize)]
pub struct MailboxesTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub colors: MailboxColors,
    pub mailboxes: Vec<JmapMailbox>,
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
                    .add_cell(Cell::new(r.id.as_deref().unwrap_or("Unknown")).fg(self.colors.id))
                    .add_cell(
                        Cell::new(r.name.as_deref().unwrap_or("Unknown")).fg(self.colors.name),
                    )
                    .add_cell(match r.role.as_ref() {
                        Some(r) => Cell::new(r.to_string()),
                        None => Cell::new(""),
                    })
                    .add_cell(Cell::new(r.total_emails).fg(self.colors.total))
                    .add_cell(Cell::new(r.unread_emails).fg(self.colors.unread))
                    .add_cell(Cell::new(if r.is_subscribed { "yes" } else { "" }));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

/// Standard JMAP mailbox role (RFC 8621 / IANA).
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "lower")]
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
}

impl From<RoleArg> for JmapMailboxRole {
    fn from(arg: RoleArg) -> Self {
        match arg {
            RoleArg::Inbox => JmapMailboxRole::Inbox,
            RoleArg::Archive => JmapMailboxRole::Archive,
            RoleArg::Drafts => JmapMailboxRole::Drafts,
            RoleArg::Flagged => JmapMailboxRole::Flagged,
            RoleArg::Important => JmapMailboxRole::Important,
            RoleArg::Junk => JmapMailboxRole::Junk,
            RoleArg::Sent => JmapMailboxRole::Sent,
            RoleArg::Subscribed => JmapMailboxRole::Subscribed,
            RoleArg::Trash => JmapMailboxRole::Trash,
        }
    }
}

/// Resolves a standard `--role` or a free-form `--custom-role` into a
/// JMAP mailbox role, when either is set. The two are mutually
/// exclusive at the clap layer.
pub(crate) fn role_from_args(
    role: Option<RoleArg>,
    custom: Option<String>,
) -> Option<JmapMailboxRole> {
    match (role, custom) {
        (Some(role), _) => Some(role.into()),
        (None, Some(custom)) => Some(JmapMailboxRole::Other(custom)),
        (None, None) => None,
    }
}

/// CLI sort key for mailboxes.
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

impl From<SortArg> for JmapMailboxSortProperty {
    fn from(arg: SortArg) -> Self {
        match arg {
            SortArg::Name => JmapMailboxSortProperty::Name,
            SortArg::SortOrder => JmapMailboxSortProperty::SortOrder,
            SortArg::ParentId => JmapMailboxSortProperty::ParentId,
        }
    }
}
