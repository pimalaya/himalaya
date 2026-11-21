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
        query::{JmapEmailComparator, JmapEmailFilter, JmapEmailSortProperty},
    },
};
use pimalaya_cli::{
    clap::args::PageTokenArg,
    listing::{Listing, ListingPage},
    printer::Printer,
};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    account::context::Account,
    error::{ErrorCode, ItemError, page_token_invalid, partial_when},
    jmap::{
        backend::{envelope_from, not_found_errors, parse_rfc3339, query_page_follows},
        client::JmapClient,
        raw,
    },
    unified::{backend::Backend, envelope::list::envelope_cells},
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
    #[arg(conflicts_with = "page_token")]
    pub page: u64,
    #[command(flatten)]
    pub page_token: PageTokenArg,
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
        let page = match &self.page_token.token {
            Some(token) => token
                .parse()
                .ok()
                .filter(|page: &u64| *page > 0)
                .ok_or_else(|| page_token_invalid(token))?,
            None => self.page.max(1),
        };

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

        let sort = vec![JmapEmailComparator {
            property: self.sort.into(),
            is_ascending: Some(!self.desc),
            collation: None,
            keyword: None,
        }];

        let position = (page - 1).saturating_mul(self.page_size);
        let args = raw::query_args(filter, Some(sort), position, self.page_size)?;
        let output = raw::query(client, "Email", args)?;

        let total = output.rest.get("total").and_then(Value::as_u64);
        let listed = output.list.len() + output.not_found.len();
        let follows = query_page_follows(total, listed, page, self.page_size);

        let mut emails = JmapEmailQueryOutput::new(account, output.list, output.rest);
        emails.mailbox = mailbox;
        emails.page = Some((page, self.page_size));
        emails.next_page = follows.then(|| (page + 1).to_string());
        emails.errors = not_found_errors(output.not_found, ErrorCode::MessageNotFound);

        printer.out(&emails)?;
        partial_when(&emails.errors)
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
    /// The emails, each as the server sent it, in its order.
    #[schemars(with = "Vec<Map<String, Value>>")]
    pub emails: Vec<Value>,
    /// The rest of the server's answers: the query's `queryState`,
    /// `canCalculateChanges`, `position` and `total`, the get's `state`.
    #[serde(flatten)]
    pub rest: Map<String, Value>,
    /// The number of the next page, `null` once the query is complete and
    /// for a get, given back through `--page-token`.
    pub next_page: Option<String>,
    /// The error of each email the server did not find (`notFound`).
    pub errors: Vec<ItemError>,
}

impl JmapEmailQueryOutput {
    /// The emails of `account`, neither filtered nor paged, beside the
    /// rest of the answers.
    pub fn new(account: &Account, emails: Vec<Value>, rest: Map<String, Value>) -> Self {
        Self {
            account: account.name().to_string(),
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
            mailbox: None,
            page: None,
            emails,
            rest,
            next_page: None,
            errors: Vec::new(),
        }
    }
}

impl fmt::Display for JmapEmailQueryOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Jmap.as_str()));

        if let Some(mailbox) = &self.mailbox {
            listing.head("Mailbox", mailbox);
        }

        for email in &self.emails {
            let email = email_of(email);
            let mut envelope = envelope_from(email.clone());
            envelope.date = email.received_at.as_deref().and_then(parse_rfc3339);

            let fmt = &self.datetime_fmt;
            let cells = envelope_cells(&envelope, fmt, self.datetime_local_tz);
            listing.items.push(cells);
        }

        if let Some((page, size)) = self.page {
            let count = self.emails.len();
            let page = ListingPage::Numbered { page, size };
            let next = self.next_page.as_deref();
            listing.page(page, count, ["email", "emails"], next);
        }

        for error in &self.errors {
            listing.errors.push(error);
        }

        write!(f, "{listing}")
    }
}

/// The email a server sent, read for the text; one that does not read
/// keeps its id alone.
pub(crate) fn email_of(email: &Value) -> JmapEmail {
    serde_json::from_value(email.clone()).unwrap_or_else(|_| JmapEmail {
        id: email.get("id").and_then(Value::as_str).map(String::from),
        ..Default::default()
    })
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

#[cfg(test)]
mod tests {
    use crate::jmap::email::query::JmapEmailQueryOutput;

    /// An exactly full last page names no next page, in JSON as in text.
    #[test]
    fn a_full_last_page_names_no_next_page() {
        let email = |id: &str| serde_json::json!({"id": id});
        let output = JmapEmailQueryOutput {
            account: String::from("work"),
            datetime_fmt: String::from("%F"),
            datetime_local_tz: false,
            mailbox: None,
            page: Some((2, 2)),
            emails: vec![email("E3"), email("E4")],
            rest: Default::default(),
            next_page: None,
            errors: Vec::new(),
        };

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["nextPage"], serde_json::Value::Null);
        let text = output.to_string();
        assert!(text.contains("2, emails 3 to 4, the last"), "{text}");
        assert!(!text.contains("Next page:"), "{text}");
    }
}
