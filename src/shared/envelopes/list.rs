use std::{collections::BTreeSet, fmt};

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Local};
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use humansize::{format_size, BINARY};
use io_email::{address::Address, envelope::Envelope, flag::Flag};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::shared::{client::EmailClient, mailboxes::arg::MailboxArg};

/// List envelopes for the active account, regardless of the underlying
/// backend (IMAP, JMAP or Maildir).
///
/// Envelopes are ordered by date descending (most recent first). Use
/// `envelope search` to filter and/or sort with the shared search
/// query DSL.
#[derive(Debug, Parser)]
pub struct EnvelopeListCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,

    /// Page number, starting from 1. The most recent envelopes are on
    /// page 1.
    #[arg(long, short = 'p')]
    #[arg(value_name = "N", default_value = "1")]
    pub page: u32,

    /// Maximum number of envelopes per page.
    ///
    /// When omitted, the merged `envelope.list.page-size` config
    /// value is used; when neither is set, the hard fallback is 25.
    #[arg(long = "page-size", short = 's')]
    #[arg(value_name = "N")]
    pub page_size: Option<u32>,

    /// Maximum width of the rendered table, in terminal columns.
    ///
    /// Overrides comfy-table's auto-detection. Columns shrink with
    /// ellipsis if needed.
    #[arg(long = "max-width", short = 'w')]
    #[arg(value_name = "COLUMNS")]
    pub max_width: Option<u16>,

    /// Render recipients (`To:`) instead of senders (`From:`). Useful
    /// for sent folders.
    #[arg(long, short)]
    pub recipient: bool,

    /// Populate the ATT column. Free on JMAP; on IMAP this fetches
    /// `BODYSTRUCTURE` in addition to `ENVELOPE`; Maildir already
    /// parses the message body for subject/from/to so the toggle is
    /// essentially free there.
    #[arg(long = "has-attachment")]
    pub has_attachment: bool,
}

impl EnvelopeListCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let page = Some(self.page).filter(|p| *p > 0);
        let page_size = self
            .page_size
            .or(Some(client.account.envelopes_list_page_size()))
            .filter(|p| *p > 0);
        let mailbox = self.mailbox.resolve(&client.account)?;

        let envelopes = client.list_envelopes(&mailbox, page, page_size, self.has_attachment)?;

        let envelopes = Envelopes {
            preset: client.account.table_preset().to_string(),
            arrangement: client.account.table_arrangement(),
            max_width: self.max_width,
            datetime_fmt: client.account.datetime_fmt().to_string(),
            datetime_local_tz: client.account.datetime_local_tz(),
            recipient: self.recipient,
            with_attachment: self.has_attachment,
            envelopes,
        };

        printer.out(envelopes)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Envelopes {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    #[serde(skip)]
    pub max_width: Option<u16>,
    #[serde(skip)]
    pub datetime_fmt: String,
    #[serde(skip)]
    pub datetime_local_tz: bool,
    #[serde(skip)]
    pub recipient: bool,
    #[serde(skip)]
    pub with_attachment: bool,
    pub envelopes: Vec<Envelope>,
}

impl fmt::Display for Envelopes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        let mut header = vec![Cell::new("ID"), Cell::new("FLAGS")];
        if self.with_attachment {
            header.push(Cell::new("ATT"));
        }
        header.push(Cell::new("SUBJECT"));
        header.push(Cell::new(if self.recipient { "TO" } else { "FROM" }));
        header.push(Cell::new("DATE"));
        header.push(Cell::new("SIZE"));

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from(header))
            .add_rows(self.envelopes.iter().map(|env| {
                let mut row = Row::new();
                row.max_height(1);
                row.add_cell(Cell::new(&env.id));
                row.add_cell(Cell::new(format_flags(&env.flags)));
                if self.with_attachment {
                    row.add_cell(Cell::new(format_attachment(env.has_attachment)));
                }
                row.add_cell(Cell::new(&env.subject));

                let addresses = if self.recipient { &env.to } else { &env.from };
                row.add_cell(Cell::new(format_addresses(addresses)));

                row.add_cell(Cell::new(format_date(
                    env.date,
                    &self.datetime_fmt,
                    self.datetime_local_tz,
                )));
                row.add_cell(Cell::new(format_size(env.size, BINARY)));
                row
            }));

        if let Some(width) = self.max_width {
            table.set_width(width);
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

/// 4-character flag widget: one slot per LCD variant. Unread (no
/// `Seen`) shows `N` in the first slot since unread is the
/// attention-grabbing case.
pub(super) fn format_flags(flags: &BTreeSet<Flag>) -> String {
    let mut out = String::with_capacity(4);
    out.push(if flags.contains(&Flag::Seen) {
        ' '
    } else {
        'N'
    });
    out.push(if flags.contains(&Flag::Answered) {
        'r'
    } else {
        ' '
    });
    out.push(if flags.contains(&Flag::Flagged) {
        '*'
    } else {
        ' '
    });
    out.push(if flags.contains(&Flag::Draft) {
        'D'
    } else {
        ' '
    });
    out
}

pub(super) fn format_attachment(has: Option<bool>) -> &'static str {
    match has {
        Some(true) => "@",
        Some(false) => "",
        None => "?",
    }
}

pub(super) fn format_addresses(addrs: &[Address]) -> String {
    addrs
        .iter()
        .map(|a| match &a.name {
            Some(name) if !name.is_empty() => name.clone(),
            _ => a.email.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn format_date(
    date: Option<DateTime<FixedOffset>>,
    fmt: &str,
    local_tz: bool,
) -> String {
    let Some(date) = date else {
        return String::new();
    };
    if local_tz {
        date.with_timezone(&Local).format(fmt).to_string()
    } else {
        date.format(fmt).to_string()
    }
}
