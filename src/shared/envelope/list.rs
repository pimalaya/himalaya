use std::{collections::BTreeSet, fmt};

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Local};
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use humansize::{BINARY, format_size};
use io_email::{address::Address, envelope::types::Envelope, flag::types::Flag};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::shared::{client::EmailClient, mailbox::arg::MailboxArg};

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
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let page = Some(self.page).filter(|p| *p > 0);
        let page_size = self
            .page_size
            .or(Some(account.envelopes_list_page_size()))
            .filter(|p| *p > 0);
        let mailbox = self.mailbox.resolve(account)?;

        let envelopes = client.list_envelopes(&mailbox, page, page_size, self.has_attachment)?;

        let envelopes = Envelopes {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            max_width: self.max_width,
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
            recipient: self.recipient,
            with_attachment: self.has_attachment,
            chars: FlagChars {
                unseen: account.envelopes_list_table_unseen_char(),
                replied: account.envelopes_list_table_replied_char(),
                flagged: account.envelopes_list_table_flagged_char(),
                attachment: account.envelopes_list_table_attachment_char(),
            },
            colors: EnvelopeColors {
                id: account.envelopes_list_table_id_color(),
                flags: account.envelopes_list_table_flags_color(),
                att: account.envelopes_list_table_att_color(),
                subject: account.envelopes_list_table_subject_color(),
                from: account.envelopes_list_table_from_color(),
                to: account.envelopes_list_table_to_color(),
                date: account.envelopes_list_table_date_color(),
                size: account.envelopes_list_table_size_color(),
            },
            envelopes,
        };

        printer.out(envelopes)
    }
}

/// Glyphs the FLAGS / ATT columns substitute in, sourced from the
/// merged account config (v1.2.0 defaults: `*`, `R`, `!`, `@`).
#[derive(Clone, Copy, Debug)]
pub(super) struct FlagChars {
    pub unseen: char,
    pub replied: char,
    pub flagged: char,
    pub attachment: char,
}

/// Per-column foreground colors for the envelopes table. `Color::Reset`
/// means "use the terminal default" (i.e. no override).
#[derive(Clone, Copy, Debug)]
pub(super) struct EnvelopeColors {
    pub id: Color,
    pub flags: Color,
    pub att: Color,
    pub subject: Color,
    pub from: Color,
    pub to: Color,
    pub date: Color,
    pub size: Color,
}

/// Table of envelope rows rendered to the terminal or as JSON.
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
    #[serde(skip)]
    pub(super) chars: FlagChars,
    #[serde(skip)]
    pub(super) colors: EnvelopeColors,
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
                row.add_cell(Cell::new(&env.id).fg(self.colors.id));
                row.add_cell(
                    Cell::new(format_flags(&env.flags, &self.chars)).fg(self.colors.flags),
                );
                if self.with_attachment {
                    row.add_cell(
                        Cell::new(format_attachment(env.has_attachment, self.chars.attachment))
                            .fg(self.colors.att),
                    );
                }
                row.add_cell(Cell::new(&env.subject).fg(self.colors.subject));

                let addresses = if self.recipient { &env.to } else { &env.from };
                let from_or_to_color = if self.recipient {
                    self.colors.to
                } else {
                    self.colors.from
                };
                row.add_cell(Cell::new(format_addresses(addresses)).fg(from_or_to_color));

                row.add_cell(
                    Cell::new(format_date(
                        env.date,
                        &self.datetime_fmt,
                        self.datetime_local_tz,
                    ))
                    .fg(self.colors.date),
                );
                row.add_cell(Cell::new(format_size(env.size, BINARY)).fg(self.colors.size));
                row
            }));

        if let Some(width) = self.max_width {
            table.set_width(width);
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

/// 3-character flag widget: unseen, replied, flagged. Each slot is a
/// space when the flag is absent, otherwise the configured glyph
/// (v1.2.0 defaults: `*`, `R`, `!`).
pub(super) fn format_flags(flags: &BTreeSet<Flag>, chars: &FlagChars) -> String {
    let mut out = String::with_capacity(3);
    out.push(if flags.iter().any(Flag::is_seen) {
        ' '
    } else {
        chars.unseen
    });
    out.push(if flags.iter().any(Flag::is_answered) {
        chars.replied
    } else {
        ' '
    });
    out.push(if flags.iter().any(Flag::is_flagged) {
        chars.flagged
    } else {
        ' '
    });
    out
}

pub(super) fn format_attachment(has: Option<bool>, glyph: char) -> String {
    match has {
        Some(true) => glyph.to_string(),
        Some(false) => String::new(),
        None => "?".to_string(),
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
