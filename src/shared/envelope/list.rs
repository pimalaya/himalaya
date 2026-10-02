//! # Envelope list
//!
//! The `envelope list` command, tabling one page of a mailbox.

use std::{collections::BTreeSet, fmt};

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Local, NaiveDate};
use clap::Parser;
use humansize::{BINARY, format_size};
use pimalaya_cli::printer::Printer;
use pimalaya_cli::table::{
    Cell, CellAlignment, Color, ColumnConstraint, ContentArrangement, Row, Table,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::{address::Address, envelope::Envelope, flag::Flag},
    shared::{
        client::EmailClient,
        mailbox::arg::MailboxArg,
        table::{printable, style_from_preset},
    },
};

/// List the envelopes of a mailbox, most recent first.
///
/// `envelope search` is the same listing with a filter and a sort.
#[derive(Debug, Parser)]
pub struct EnvelopeListCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Page number, starting at 1, which holds the most recent
    /// envelopes.
    #[arg(long, short = 'p')]
    #[arg(value_name = "N", default_value = "1")]
    pub page: u32,
    /// Maximum number of envelopes per page.
    ///
    /// Omitted, the configured `envelope.list.page-size` answers, and 25
    /// is the hard fallback.
    #[arg(long = "page-size", short = 's')]
    #[arg(value_name = "N")]
    pub page_size: Option<u32>,
    /// Maximum width of the rendered table, in terminal columns.
    ///
    /// Overrides the auto-detected width, columns shrinking with an
    /// ellipsis as needed.
    #[arg(long = "max-width", short = 'w')]
    #[arg(value_name = "COLUMNS")]
    pub max_width: Option<u16>,
    /// Render recipients instead of senders, which is what a sent
    /// mailbox wants.
    #[arg(long, short)]
    pub recipient: bool,
    /// Fill the ATT column, which on some backends costs one extra
    /// lookup per envelope.
    #[arg(long = "has-attachment")]
    pub has_attachment: bool,
}

impl EnvelopeListCommand {
    /// Lists one page of the mailbox and prints it as a table.
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
        let mailbox = self.mailbox.resolve(account);

        let envelopes = client.list_envelopes(&mailbox, page, page_size, self.has_attachment)?;
        let queued = client.queued_messages(&mailbox)?;

        let envelopes = Envelopes {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            max_width: self.max_width,
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
            datetime_relative: account.datetime_relative(),
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
            queued,
            envelopes,
        };

        printer.out(envelopes)
    }
}

/// Glyphs the FLAGS and ATT columns are drawn with, from the merged
/// account configuration.
#[derive(Clone, Copy, Debug)]
pub struct FlagChars {
    /// Glyph of a message lacking `\Seen`.
    pub unseen: char,
    /// Glyph of a message carrying `\Answered`.
    pub replied: char,
    /// Glyph of a message carrying `\Flagged`.
    pub flagged: char,
    /// Glyph of a message carrying an attachment.
    pub attachment: char,
}

/// Per-column colors of the envelopes table, `Color::Reset` leaving the
/// terminal default in place.
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

/// The `envelope list` output, a table of envelopes.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Envelopes {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The column arrangement the table renders with.
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    /// The width the table is capped at, when one was asked for.
    #[serde(skip)]
    pub max_width: Option<u16>,
    /// The chrono `strftime` format of the DATE column.
    #[serde(skip)]
    pub datetime_fmt: String,
    /// Whether a date is converted to the local timezone first.
    #[serde(skip)]
    pub datetime_local_tz: bool,
    /// Whether a recent date renders relative to today.
    #[serde(skip)]
    pub datetime_relative: bool,
    /// Whether recipients are drawn instead of senders.
    #[serde(skip)]
    pub recipient: bool,
    /// Whether the ATT column is drawn.
    #[serde(skip)]
    pub with_attachment: bool,
    #[serde(skip)]
    pub(super) chars: FlagChars,
    #[serde(skip)]
    pub(super) colors: EnvelopeColors,
    /// Messages staged for creation and not pushed yet, which have no id
    /// and so no row.
    ///
    /// Zero for every backend whose writes reach the server as they are
    /// made.
    pub queued: usize,
    /// The envelopes of this page.
    pub envelopes: Vec<Envelope>,
}

impl fmt::Display for Envelopes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let today = Local::now().date_naive();
        let mut table = Table::new();

        let mut header = vec![Cell::new("ID"), Cell::new("FLAGS")];
        if self.with_attachment {
            header.push(Cell::new("ATT"));
        }
        header.push(Cell::new("SUBJECT"));
        header.push(Cell::new(if self.recipient { "TO" } else { "FROM" }));
        header.push(Cell::new("DATE"));
        header.push(Cell::new("SIZE").set_alignment(CellAlignment::Right));
        let mut header = Row::from(header);
        header.max_height(1);

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(header)
            .add_rows(self.envelopes.iter().map(|env| {
                let mut row = Row::new();
                row.max_height(1);
                row.add_cell(Cell::new(printable(&env.id)).fg(self.colors.id));
                row.add_cell(
                    Cell::new(format_flags(&env.flags, &self.chars)).fg(self.colors.flags),
                );
                if self.with_attachment {
                    row.add_cell(
                        Cell::new(format_attachment(env.has_attachment, self.chars.attachment))
                            .fg(self.colors.att),
                    );
                }
                row.add_cell(Cell::new(printable(&env.subject)).fg(self.colors.subject));

                let addresses = if self.recipient { &env.to } else { &env.from };
                let from_or_to_color = if self.recipient {
                    self.colors.to
                } else {
                    self.colors.from
                };
                row.add_cell(
                    Cell::new(printable(&format_addresses(addresses))).fg(from_or_to_color),
                );

                row.add_cell(Cell::new(self.format_date(env.date, today)).fg(self.colors.date));
                row.add_cell(
                    Cell::new(format_size(env.size, BINARY))
                        .fg(self.colors.size)
                        .set_alignment(CellAlignment::Right),
                );
                row
            }));

        // NOTE: an id is what a follow-up command takes, so squeezing
        // the table into `--max-width` truncates any column but this one.
        if let Some(column) = table.column_mut(0) {
            column.set_constraint(ColumnConstraint::ContentWidth);
        }

        if let Some(width) = self.max_width {
            table.set_width(width);
        }

        writeln!(f)?;
        writeln!(f, "{table}")?;

        // NOTE: a queued message has no row, so saying how many there are
        // is what keeps a saved one from reading as a lost one.
        match self.queued {
            0 => Ok(()),
            1 => writeln!(f, "1 queued message, see `himalaya pimdir queue list`"),
            n => writeln!(f, "{n} queued messages, see `himalaya pimdir queue list`"),
        }
    }
}

/// Renders the three-slot FLAGS widget: unseen, replied, flagged.
///
/// A slot is a space when its flag is absent and the configured glyph
/// when it is set.
pub fn format_flags(flags: &BTreeSet<Flag>, chars: &FlagChars) -> String {
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

/// Renders the ATT cell: the glyph, nothing, or `?` when the backend
/// could not tell.
pub(super) fn format_attachment(has: Option<bool>, glyph: char) -> String {
    match has {
        Some(true) => glyph.to_string(),
        Some(false) => String::new(),
        None => "?".to_string(),
    }
}

/// Renders addresses as a comma-separated list of display names, falling
/// back to the address itself.
pub fn format_addresses(addrs: &[Address]) -> String {
    addrs
        .iter()
        .map(|a| match &a.name {
            Some(name) if !name.is_empty() => name.clone(),
            _ => a.email.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

impl Envelopes {
    /// Renders a date with the configured format, in the local timezone
    /// when asked for, or relative to `today` when recent and asked for.
    fn format_date(&self, date: Option<DateTime<FixedOffset>>, today: NaiveDate) -> String {
        let Some(date) = date else {
            return String::new();
        };

        if self.datetime_relative {
            let date = date.with_timezone(&Local);

            return match (today - date.date_naive()).num_days() {
                0 => date.format("%R").to_string(),
                1 => String::from("yesterday"),
                2..7 => date.format("%A").to_string(),
                _ => date.format(&self.datetime_fmt).to_string(),
            };
        }

        if self.datetime_local_tz {
            date.with_timezone(&Local)
                .format(&self.datetime_fmt)
                .to_string()
        } else {
            date.format(&self.datetime_fmt).to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, FixedOffset, Local, NaiveDate, TimeZone};
    use pimalaya_cli::table::{Color, ContentArrangement};

    use super::{Address, Envelope, EnvelopeColors, Envelopes, FlagChars};

    fn envelopes(relative: bool) -> Envelopes {
        Envelopes {
            preset: String::new(),
            arrangement: ContentArrangement::Disabled,
            max_width: None,
            datetime_fmt: String::from("%F"),
            datetime_local_tz: false,
            datetime_relative: relative,
            recipient: false,
            with_attachment: false,
            chars: FlagChars {
                unseen: '*',
                replied: 'R',
                flagged: '!',
                attachment: '@',
            },
            colors: EnvelopeColors {
                id: Color::Reset,
                flags: Color::Reset,
                att: Color::Reset,
                subject: Color::Reset,
                from: Color::Reset,
                to: Color::Reset,
                date: Color::Reset,
                size: Color::Reset,
            },
            queued: 0,
            envelopes: Vec::new(),
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()
    }

    fn local(y: i32, m: u32, d: u32, h: u32, min: u32) -> Option<DateTime<FixedOffset>> {
        let naive = NaiveDate::from_ymd_opt(y, m, d)?.and_hms_opt(h, min, 0)?;
        Some(Local.from_local_datetime(&naive).single()?.fixed_offset())
    }

    #[test]
    fn relative_date_follows_age() {
        let envelopes = envelopes(true);
        let render = |date| envelopes.format_date(date, today());

        assert_eq!(render(local(2026, 9, 30, 11, 35)), "11:35");
        assert_eq!(render(local(2026, 9, 29, 11, 35)), "yesterday");
        assert_eq!(render(local(2026, 9, 26, 11, 35)), "Saturday");
        assert_eq!(render(local(2026, 9, 23, 11, 35)), "2026-09-23");
        assert_eq!(render(None), "");
    }

    #[test]
    fn control_characters_from_the_sender_are_not_printed() {
        let mut envelopes = envelopes(false);
        envelopes.envelopes.push(Envelope {
            id: String::from("1"),
            message_id: None,
            in_reply_to: Vec::new(),
            flags: Default::default(),
            // Writes the clipboard (OSC 52), then clears the screen.
            subject: String::from("Hi\x1b]52;c;ZWNobyBoaQ==\x07\x1b[2J"),
            from: vec![Address {
                name: Some(String::from("Eve\x1b[8m")),
                email: String::from("eve@example.org"),
            }],
            to: Vec::new(),
            date: None,
            size: 0,
            has_attachment: None,
        });

        let output = envelopes.to_string();

        assert!(!output.contains('\x1b'), "{output:?}");
        assert!(!output.contains('\x07'), "{output:?}");
        assert!(output.contains("Hi"), "{output:?}");
    }

    #[test]
    fn absolute_date_ignores_age() {
        let envelopes = envelopes(false);
        let date = local(2026, 9, 30, 11, 35);
        let expected = date.unwrap().format("%F").to_string();

        assert_eq!(envelopes.format_date(date, today()), expected);
    }
}
