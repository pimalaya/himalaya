//! # JMAP mailbox query
//!
//! The `jmap mailbox query` command, an RFC 8621 `Mailbox/query` chained
//! into a `Mailbox/get`.

use std::fmt;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use io_jmap::rfc8621::mailbox::{
    JmapMailbox, JmapMailboxRole,
    query::{
        JmapMailboxFilter, JmapMailboxQueryOptions, JmapMailboxSortComparator,
        JmapMailboxSortProperty,
    },
};
use pimalaya_cli::{
    clap::args::PageTokenArg,
    listing::{Listing, ListingPage},
    printer::Printer,
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    error::{ErrorCode, ItemError, partial_when},
    jmap::{
        backend::{get_errors, query_page_follows},
        client::JmapClient,
    },
    listing,
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
    #[command(flatten)]
    pub page_token: PageTokenArg,
}

impl JmapMailboxQueryCommand {
    /// Queries the mailboxes and prints the page it returned.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let page = match &self.page_token.token {
            Some(token) => token
                .parse()
                .ok()
                .filter(|page: &u64| *page > 0)
                .with_context(|| format!("Invalid page token {token}"))?,
            None => self.page.max(1),
        };

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
            position: Some((page - 1).saturating_mul(self.page_size)),
            limit: Some(self.page_size),
            properties: None,
        })?;

        let listed = output.mailboxes.len() + output.unparsed.len() + output.not_found.len();
        let follows = query_page_follows(output.total, listed, page, self.page_size);

        let mailboxes = JmapMailboxQueryOutput {
            account: account.name().to_string(),
            page: Some((page, self.page_size)),
            mailboxes: output.mailboxes,
            next_page: follows.then(|| (page + 1).to_string()),
            errors: get_errors(
                output.not_found,
                output.unparsed,
                ErrorCode::MailboxNotFound,
                ErrorCode::MailboxUnreadable,
            ),
        };

        printer.out(&mailboxes)?;
        partial_when(&mailboxes.errors)
    }
}

/// The `jmap mailbox query` and `get` output, the mailboxes the server
/// returned.
#[derive(Clone, Debug, Default, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JmapMailboxQueryOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The page listed and its size, `None` for a get.
    #[serde(skip)]
    pub page: Option<(u64, u64)>,
    /// The mailboxes, in the order the server returned them.
    pub mailboxes: Vec<JmapMailbox>,
    /// The number of the next page, `null` once the query is complete and
    /// for a get, given back through `--page-token`.
    pub next_page: Option<String>,
    /// The error of each mailbox the server did not find or that did not
    /// deserialize, in the order met.
    pub errors: Vec<ItemError>,
}

impl fmt::Display for JmapMailboxQueryOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Jmap.as_str()));

        for mailbox in &self.mailboxes {
            let text = |value: &Option<String>| {
                sanitize::text(value.as_deref().unwrap_or("Unknown")).into_owned()
            };
            let role = mailbox.role.as_ref().map(ToString::to_string);

            listing.items.push([
                text(&mailbox.id),
                role.unwrap_or_default(),
                listing::counts(
                    Some(u64::from(mailbox.unread_emails)),
                    Some(u64::from(mailbox.total_emails)),
                ),
                String::from(if mailbox.is_subscribed {
                    "subscribed"
                } else {
                    ""
                }),
                text(&mailbox.name),
            ]);
        }

        if let Some((page, size)) = self.page {
            let count = self.mailboxes.len();
            let page = ListingPage::Numbered { page, size };
            let next = self.next_page.as_deref();
            listing.page(page, count, ["mailbox", "mailboxes"], next);
        }

        for error in &self.errors {
            listing.errors.push(error);
        }

        write!(f, "{listing}")
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

#[cfg(test)]
mod tests {
    use io_jmap::rfc8621::mailbox::JmapMailbox;

    use crate::jmap::mailbox::query::JmapMailboxQueryOutput;

    /// A page names the next one only when the query says one follows,
    /// never because it came back full: `nextPage` in JSON, `Next page:`
    /// in text.
    #[test]
    fn a_full_page_names_the_next_only_when_one_follows() {
        let mailbox = |id: &str| JmapMailbox {
            id: Some(id.to_owned()),
            name: Some(id.to_owned()),
            ..Default::default()
        };
        let mut output = JmapMailboxQueryOutput {
            account: String::from("work"),
            page: Some((1, 2)),
            mailboxes: vec![mailbox("M1"), mailbox("M2")],
            next_page: None,
            errors: Vec::new(),
        };

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["nextPage"], serde_json::Value::Null);
        let text = output.to_string();
        assert!(text.contains("1, mailboxes 1 to 2, the last"), "{text}");
        assert!(!text.contains("Next page:"), "{text}");

        output.next_page = Some(String::from("2"));
        assert_eq!(serde_json::to_value(&output).unwrap()["nextPage"], "2");
        let text = output.to_string();
        assert!(text.contains("Next page:  --page 2"), "{text}");
    }
}
