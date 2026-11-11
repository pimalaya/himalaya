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
        JmapEmail, JmapEmailAddress,
        query::{
            JmapEmailComparator, JmapEmailFilter, JmapEmailQueryOptions, JmapEmailSortProperty,
        },
    },
};
use pimalaya_cli::{
    fields::Fields,
    printer::Printer,
    records::{Record, Records},
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::jmap::client::JmapClient;

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
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
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

        let emails = JmapEmailQueryOutput {
            emails: output.emails,
        };

        printer.out(emails)
    }
}

/// The `jmap email query` output, the envelopes of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JmapEmailQueryOutput {
    /// The emails, in the order the server returned them.
    pub emails: Vec<JmapEmail>,
}

impl fmt::Display for JmapEmailQueryOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut records = Records::default();

        for e in &self.emails {
            let keyword = |name: &str| {
                let keywords = e.keywords.as_ref();
                keywords.and_then(|k| k.get(name)).copied().unwrap_or(false)
            };

            let mut state = Vec::new();
            if !keyword("$seen") {
                state.push(String::from("unread"));
            }
            if keyword("$answered") {
                state.push(String::from("answered"));
            }
            if keyword("$flagged") {
                state.push(String::from("flagged"));
            }
            if e.has_attachment == Some(true) {
                state.push(String::from("attachment"));
            }

            let from = format_addresses(e.from.as_deref().unwrap_or_default());
            let date = e.received_at.as_deref().unwrap_or_default();

            let mut fields = Fields::default();
            fields.push("From", sanitize::text(&from));
            fields.push("Date", sanitize::text(date));

            records.push(Record {
                title: sanitize::text(e.id.as_deref().unwrap_or_default()).into_owned(),
                state,
                fields,
                body: sanitize::text(e.subject.as_deref().unwrap_or_default()).into_owned(),
            });
        }

        write!(f, "{records}")
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

/// Renders addresses as a comma-separated list of `Name <email>`, or the
/// bare email when it has no name.
fn format_addresses(addrs: &[JmapEmailAddress]) -> String {
    addrs
        .iter()
        .map(|a| match &a.name {
            Some(name) if !name.is_empty() => format!("{name} <{}>", a.email),
            _ => a.email.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
