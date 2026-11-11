//! # Envelope list
//!
//! The `envelope list` command, listing one page of a mailbox.

use std::{collections::BTreeSet, fmt};

use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset, Local, NaiveDate};
use clap::Parser;
use humansize::{BINARY, format_size};
use pimalaya_cli::{
    clap::args::PageTokenArg, columns::Columns, output::Paginated, printer::Printer, sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::{address::Address, envelope::Envelope, flag::Flag},
    shared::{client::EmailClient, mailbox::arg::MailboxArg},
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
    #[command(flatten)]
    pub page_token: PageTokenArg,
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
    /// Lists one page of the mailbox and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let page = match &self.page_token.token {
            Some(token) => token
                .parse()
                .with_context(|| format!("Invalid page token {token}"))?,
            None => self.page,
        };
        let page = Some(page).filter(|p| *p > 0);
        let page_size = self
            .page_size
            .or(Some(account.envelopes_list_page_size()))
            .filter(|p| *p > 0);
        let mailbox = self.mailbox.resolve(account);

        let envelopes = client.list_envelopes(&mailbox, page, page_size, self.has_attachment)?;

        // NOTE: the token is the next page number, given while a page comes
        // back full; the page after a full last one comes back empty.
        let next_page = match (page, page_size) {
            (Some(page), Some(size)) if envelopes.len() >= size as usize => {
                Some((page + 1).to_string())
            }
            _ => None,
        };

        let envelopes = EnvelopeListOutput {
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
            datetime_relative: account.datetime_relative(),
            recipient: self.recipient,
            with_attachment: self.has_attachment,
            envelopes,
        };

        printer.out(Paginated {
            inner: envelopes,
            next_page,
        })
    }
}

/// FLAGS glyph of a message lacking `\Seen`.
pub const UNSEEN_GLYPH: char = '*';
/// FLAGS glyph of a message carrying `\Answered`.
pub const REPLIED_GLYPH: char = 'R';
/// FLAGS glyph of a message carrying `\Flagged`.
pub const FLAGGED_GLYPH: char = '!';
/// ATT glyph of a message carrying an attachment.
pub const ATTACHMENT_GLYPH: char = '@';

/// The `envelope list` output, the envelopes of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeListOutput {
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
    /// The envelopes of this page.
    pub envelopes: Vec<Envelope>,
}

impl fmt::Display for EnvelopeListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let today = Local::now().date_naive();

        // NOTE: the subject is the one free-text column, so it reads last
        // and a long one never pushes the others out of line.
        let mut header = vec!["ID", "FLAGS"];
        if self.with_attachment {
            header.push("ATT");
        }
        header.push(if self.recipient { "TO" } else { "FROM" });
        header.extend(["DATE", "SIZE", "SUBJECT"]);

        let mut columns = Columns::new(header);

        for env in &self.envelopes {
            let addresses = if self.recipient { &env.to } else { &env.from };

            let mut row = vec![
                sanitize::text(&env.id).into_owned(),
                format_flags(&env.flags),
            ];
            if self.with_attachment {
                row.push(format_attachment(env.has_attachment));
            }
            row.push(sanitize::text(&format_addresses(addresses)).into_owned());
            row.push(self.format_date(env.date, today));
            row.push(format_size(env.size, BINARY));
            row.push(sanitize::text(&env.subject).into_owned());

            columns.push(row);
        }

        write!(f, "{columns}")
    }
}

/// Renders the FLAGS cell: the glyphs of unseen, replied and flagged, in
/// that order, each only when it applies.
///
/// No flag renders an empty cell, which a list prints as `-`.
pub fn format_flags(flags: &BTreeSet<Flag>) -> String {
    let mut out = String::with_capacity(3);
    if !flags.iter().any(Flag::is_seen) {
        out.push(UNSEEN_GLYPH);
    }
    if flags.iter().any(Flag::is_answered) {
        out.push(REPLIED_GLYPH);
    }
    if flags.iter().any(Flag::is_flagged) {
        out.push(FLAGGED_GLYPH);
    }
    out
}

/// Renders the ATT cell: the glyph, nothing, or `?` when the backend
/// could not tell.
pub(super) fn format_attachment(has: Option<bool>) -> String {
    match has {
        Some(true) => ATTACHMENT_GLYPH.to_string(),
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

impl EnvelopeListOutput {
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
    use std::collections::BTreeSet;

    use chrono::{DateTime, FixedOffset, Local, NaiveDate, TimeZone};

    use crate::{
        email::{
            address::Address,
            envelope::Envelope,
            flag::{Flag, IanaFlag},
        },
        shared::envelope::list::EnvelopeListOutput,
    };

    fn envelopes(relative: bool) -> EnvelopeListOutput {
        EnvelopeListOutput {
            datetime_fmt: String::from("%F"),
            datetime_local_tz: false,
            datetime_relative: relative,
            recipient: false,
            with_attachment: false,
            envelopes: Vec::new(),
        }
    }

    fn envelope(id: &str, flags: BTreeSet<Flag>, subject: &str, from: &str) -> Envelope {
        Envelope {
            id: String::from(id),
            message_id: None,
            in_reply_to: Vec::new(),
            flags,
            subject: String::from(subject),
            from: vec![Address {
                name: Some(String::from(from)),
                email: String::from("alice@example.org"),
            }],
            to: Vec::new(),
            date: None,
            size: 2048,
            has_attachment: None,
        }
    }

    /// The list is plain text: a header, aligned columns, no border and
    /// no colour, the subject last and a flagless message's FLAGS `-`.
    #[test]
    fn the_list_is_aligned_plain_text() {
        let mut envelopes = envelopes(false);
        let unseen_flagged = BTreeSet::from([Flag::from_iana(IanaFlag::Flagged)]);
        let seen = BTreeSet::from([Flag::from_iana(IanaFlag::Seen)]);
        envelopes.envelopes = vec![
            envelope("12", unseen_flagged, "Lunch on Friday?", "Alice"),
            envelope("3", seen, "Re: the report", "Bob Martin"),
        ];

        let expected = "\
ID  FLAGS  FROM        DATE  SIZE   SUBJECT
12  *!     Alice       -     2 KiB  Lunch on Friday?
3   -      Bob Martin  -     2 KiB  Re: the report
";
        assert_eq!(envelopes.to_string(), expected);
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
