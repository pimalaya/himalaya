use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_jmap::{
    rfc8620::JmapFilter,
    rfc8621::email::{
        JmapEmail, JmapEmailAddress, JmapEmailComparator, JmapEmailFilter, JmapEmailSortProperty,
        query::JmapEmailQueryOptions,
    },
};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
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
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
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
        })?;

        let table = EmailsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            colors: EmailsColors {
                id: account.envelopes_list_table_id_color(),
                flags: account.envelopes_list_table_flags_color(),
                subject: account.envelopes_list_table_subject_color(),
                from: account.envelopes_list_table_from_color(),
                date: account.envelopes_list_table_date_color(),
            },
            chars: EmailsChars {
                unseen: account.envelopes_list_table_unseen_char(),
                flagged: account.envelopes_list_table_flagged_char(),
                attachment: account.envelopes_list_table_attachment_char(),
            },
            emails: output.emails,
        };

        printer.out(table)
    }
}

/// Per-column colors for the emails table.
#[derive(Clone, Copy, Debug)]
pub struct EmailsColors {
    pub id: Color,
    pub flags: Color,
    pub subject: Color,
    pub from: Color,
    pub date: Color,
}

/// Flag glyphs used in the emails table.
#[derive(Clone, Copy, Debug)]
pub struct EmailsChars {
    pub unseen: char,
    pub flagged: char,
    pub attachment: char,
}

/// Renderable table of email envelopes.
#[derive(Clone, Debug, Serialize)]
pub struct EmailsTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    #[serde(skip)]
    pub colors: EmailsColors,
    #[serde(skip)]
    pub chars: EmailsChars,
    pub emails: Vec<JmapEmail>,
}

impl fmt::Display for EmailsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("FLAGS"),
                Cell::new("SUBJECT"),
                Cell::new("FROM"),
                Cell::new("DATE"),
            ]));

        for e in &self.emails {
            let mut flags = String::new();
            let kw = e.keywords.as_ref();
            if !kw.and_then(|k| k.get("$seen")).copied().unwrap_or(false) {
                flags.push(self.chars.unseen);
            }
            if kw.and_then(|k| k.get("$flagged")).copied().unwrap_or(false) {
                flags.push(self.chars.flagged);
            }
            if e.has_attachment.unwrap_or(false) {
                flags.push(self.chars.attachment);
            }

            let mut row = Row::new();
            row.max_height(1);
            row.add_cell(Cell::new(e.id.as_deref().unwrap_or("")).fg(self.colors.id));
            row.add_cell(Cell::new(&flags).fg(self.colors.flags));
            row.add_cell(Cell::new(e.subject.as_deref().unwrap_or("")).fg(self.colors.subject));
            row.add_cell(
                Cell::new(format_addresses(e.from.as_deref().unwrap_or(&[]))).fg(self.colors.from),
            );
            row.add_cell(Cell::new(e.received_at.as_deref().unwrap_or("")).fg(self.colors.date));
            table.add_row(row);
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

/// CLI sort key for emails.
#[derive(Clone, Debug, Default, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum SortArg {
    #[default]
    ReceivedAt,
    SentAt,
    Size,
    From,
    To,
    Subject,
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

fn format_addresses(addrs: &[JmapEmailAddress]) -> String {
    addrs
        .iter()
        .map(|a| {
            if let Some(name) = &a.name {
                if !name.is_empty() {
                    return name.clone();
                }
            }
            a.email.clone()
        })
        .collect::<Vec<_>>()
        .join(", ")
}
