//! # JMAP email query
//!
//! The `jmap email query` command, an RFC 8621 `Email/query` chained into
//! an `Email/get`.

use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use io_jmap::{
    rfc8620::filter::JmapFilter,
    rfc8621::email::{
        JmapEmail,
        query::{
            JmapEmailComparator, JmapEmailFilter, JmapEmailQueryOptions, JmapEmailSortProperty,
        },
    },
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    jmap::{
        backend::{envelope_from, parse_rfc3339},
        client::JmapClient,
    },
    listing::{Listing, ListingPage},
    shared::envelope::list::envelope_cells,
};

/// Query JMAP emails (Email/query + Email/get).
///
/// Lists, filters and sorts email envelopes.
#[derive(Debug, Parser)]
pub struct JmapEmailQueryCommand {
    /// Filter by mailbox ID.
    #[arg(long, short, value_name = "MAILBOX-ID")]
    pub mailbox: Option<String>,
    /// Filter by received-before date (RFC 3339, e.g. 2024-01-01T00:00:00Z).
    #[arg(long, value_name = "DATE")]
    pub before: Option<String>,
    /// Filter by received-after date (RFC 3339, e.g. 2024-01-01T00:00:00Z).
    #[arg(long, value_name = "DATE")]
    pub after: Option<String>,
    /// Filter by minimum size in bytes.
    #[arg(long, value_name = "BYTES")]
    pub min_size: Option<u64>,
    /// Filter by maximum size in bytes.
    #[arg(long, value_name = "BYTES")]
    pub max_size: Option<u64>,
    /// Filter to emails that have this keyword set.
    #[arg(long, value_name = "KEYWORD")]
    pub has_keyword: Option<String>,
    /// Filter to emails that do not have this keyword set.
    #[arg(long, value_name = "KEYWORD")]
    pub not_keyword: Option<String>,
    /// Filter to emails that have at least one attachment.
    #[arg(long)]
    pub has_attachment: bool,
    /// Full-text search across all headers and body.
    #[arg(long, value_name = "TEXT")]
    pub text: Option<String>,
    /// Filter by From header (substring match).
    #[arg(long, value_name = "TEXT")]
    pub from: Option<String>,
    /// Filter by To header (substring match).
    #[arg(long, value_name = "TEXT")]
    pub to: Option<String>,
    /// Filter by Subject header (substring match).
    #[arg(long, value_name = "TEXT")]
    pub subject: Option<String>,
    /// Filter by email body (substring match).
    #[arg(long, value_name = "TEXT")]
    pub body: Option<String>,
    /// Sort by property.
    #[arg(long, value_name = "PROP", default_value_t)]
    pub sort: SortArg,
    /// Sort in descending order.
    #[arg(long, default_value_t)]
    pub desc: bool,
    /// Number of emails to display per page.
    #[arg(long, short = 's', value_name = "N", default_value = "10")]
    pub page_size: u64,
    /// Page index, starting from 1.
    #[arg(long, short, value_name = "N", default_value = "1")]
    pub page: u64,
}

impl JmapEmailQueryCommand {
    /// Queries the emails and prints the page it returned.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.clone();

        let filter = {
            let f = JmapEmailFilter {
                in_mailbox: self.mailbox,
                before: self.before,
                after: self.after,
                min_size: self.min_size,
                max_size: self.max_size,
                has_keyword: self.has_keyword,
                not_keyword: self.not_keyword,
                has_attachment: if self.has_attachment {
                    Some(true)
                } else {
                    None
                },
                text: self.text,
                from: self.from,
                to: self.to,
                subject: self.subject,
                body: self.body,
                ..Default::default()
            };

            let has_one_filter = f.in_mailbox.is_some()
                || f.before.is_some()
                || f.after.is_some()
                || f.min_size.is_some()
                || f.max_size.is_some()
                || f.has_keyword.is_some()
                || f.not_keyword.is_some()
                || f.has_attachment.is_some()
                || f.text.is_some()
                || f.from.is_some()
                || f.to.is_some()
                || f.subject.is_some()
                || f.body.is_some();

            if has_one_filter {
                Some(JmapFilter::from(f))
            } else {
                None
            }
        };

        let sort = Some(vec![JmapEmailComparator {
            property: self.sort.into(),
            is_ascending: Some(!self.desc),
            collation: None,
            keyword: None,
        }]);

        let output = client.email_query(JmapEmailQueryOptions {
            filter,
            sort,
            position: Some(self.page.saturating_sub(1) * self.page_size),
            limit: Some(self.page_size),
            properties: None,
            ..Default::default()
        })?;

        let mut emails = JmapEmailQueryOutput::new(account, output.emails);
        emails.mailbox = mailbox;
        emails.page = Some((self.page.max(1), self.page_size));

        printer.out(emails)
    }
}

/// The `jmap email query` output, the envelopes of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JmapEmailQueryOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The chrono `strftime` format of a date.
    #[serde(skip)]
    pub datetime_fmt: String,
    /// Whether a date is converted to the local timezone first.
    #[serde(skip)]
    pub datetime_local_tz: bool,
    /// The mailbox id the query filtered on, if any.
    #[serde(skip)]
    pub mailbox: Option<String>,
    /// The page listed and its size, `None` for a get.
    #[serde(skip)]
    pub page: Option<(u64, u64)>,
    /// The emails, in the order the server returned them.
    pub emails: Vec<JmapEmail>,
}

impl JmapEmailQueryOutput {
    /// The emails of `account`, neither filtered nor paged.
    pub fn new(account: &Account, emails: Vec<JmapEmail>) -> Self {
        Self {
            account: account.name().to_string(),
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
            mailbox: None,
            page: None,
            emails,
        }
    }
}

impl fmt::Display for JmapEmailQueryOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Jmap));

        if let Some(mailbox) = &self.mailbox {
            listing.head("Mailbox", mailbox);
        }

        for email in &self.emails {
            let mut envelope = envelope_from(email.clone());
            envelope.date = email.received_at.as_deref().and_then(parse_rfc3339);

            let fmt = &self.datetime_fmt;
            let cells = envelope_cells(&envelope, fmt, self.datetime_local_tz);
            listing.items.push(cells);
        }

        if let Some((page, size)) = self.page {
            let count = self.emails.len();
            let next = (count as u64 >= size).then(|| (page + 1).to_string());
            let page = ListingPage::Numbered { page, size };
            listing.page(page, count, ["email", "emails"], next.as_deref());
        }

        write!(f, "{listing}")
    }
}

/// The property an email query sorts on.
#[derive(Clone, Debug, Default, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum SortArg {
    /// When the server received the email.
    #[default]
    ReceivedAt,
    /// When its author says it was sent.
    SentAt,
    /// Its size, in octets.
    Size,
    /// Its first `From` address.
    From,
    /// Its first `To` address.
    To,
    /// Its subject.
    Subject,
    /// Whether it carries an attachment.
    HasAttachment,
}

impl fmt::Display for SortArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReceivedAt => write!(f, "received-at"),
            Self::SentAt => write!(f, "sent-at"),
            Self::Size => write!(f, "size"),
            Self::From => write!(f, "from"),
            Self::To => write!(f, "to"),
            Self::Subject => write!(f, "subject"),
            Self::HasAttachment => write!(f, "has-attachment"),
        }
    }
}

impl From<SortArg> for JmapEmailSortProperty {
    fn from(arg: SortArg) -> Self {
        match arg {
            SortArg::ReceivedAt => JmapEmailSortProperty::ReceivedAt,
            SortArg::SentAt => JmapEmailSortProperty::SentAt,
            SortArg::Size => JmapEmailSortProperty::Size,
            SortArg::From => JmapEmailSortProperty::From,
            SortArg::To => JmapEmailSortProperty::To,
            SortArg::Subject => JmapEmailSortProperty::Subject,
            SortArg::HasAttachment => JmapEmailSortProperty::HasAttachment,
        }
    }
}
