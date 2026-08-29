//! # JMAP mailbox query
//!
//! The `jmap mailbox query` command, an RFC 8621 `Mailbox/query` chained
//! into a `Mailbox/get`.

use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use comfy_table::{Cell, Color, Row, Table};
use io_jmap::rfc8621::mailbox::{
    JmapMailbox, JmapMailboxRole,
    query::{
        JmapMailboxFilter, JmapMailboxQueryOptions, JmapMailboxSortComparator,
        JmapMailboxSortProperty,
    },
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, jmap::client::JmapClient, shared::table::style_from_preset,
};

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
    /// Restrict to subscribed mailboxes. Native `Mailbox/query` applies
    /// no subscription filter, so the default lists every mailbox.
    #[arg(long, default_value_t)]
    pub subscribed: bool,
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
    /// Queries the mailboxes and tables the page it returned.
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
                is_subscribed: if self.subscribed { Some(true) } else { None },
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

/// Per-column colors of the mailboxes table.
#[derive(Clone, Copy, Debug)]
pub struct MailboxColors {
    /// Color of the ID column.
    pub id: Color,
    /// Color of the NAME column.
    pub name: Color,
    /// Color of the TOTAL column.
    pub total: Color,
    /// Color of the UNREAD column.
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

/// The mailboxes rendered as a table.
#[derive(Clone, Debug, Default, Serialize, JsonSchema)]
pub struct MailboxesTable {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The per-column colors.
    #[serde(skip)]
    pub colors: MailboxColors,
    /// The mailboxes, in the order the server returned them.
    pub mailboxes: Vec<JmapMailbox>,
}

impl fmt::Display for MailboxesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
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

/// A registered JMAP mailbox role, per RFC 8621 and the IANA registry.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum RoleArg {
    /// The mailbox new mail arrives in.
    Inbox,
    /// The mailbox archived mail is kept in.
    Archive,
    /// The mailbox unsent mail is kept in.
    Drafts,
    /// The mailbox flagged mail is gathered in.
    Flagged,
    /// The mailbox important mail is gathered in.
    Important,
    /// The mailbox junk is gathered in.
    Junk,
    /// The mailbox sent mail is kept in.
    Sent,
    /// The mailbox subscribed feeds are delivered to.
    Subscribed,
    /// The mailbox deleted mail is kept in.
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

/// The property a mailbox query sorts on.
#[derive(Clone, Debug, Default, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum SortArg {
    /// The mailbox name.
    Name,
    /// The order the server itself wants them displayed in.
    #[default]
    SortOrder,
    /// The id of the parent mailbox.
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
