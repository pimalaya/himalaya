//! # Envelope list
//!
//! The `envelope list` command, listing one page of a mailbox.

use std::{collections::BTreeSet, fmt};

use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset, Local, NaiveDate};
use clap::Parser;
use humansize::{BINARY, format_size};
use pimalaya_cli::{
    clap::args::PageTokenArg,
    fields::Fields,
    output::Paginated,
    printer::Printer,
    records::{Record, Records},
    sanitize,
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
    /// Tell which envelopes carry an attachment, which on some backends
    /// costs one extra lookup per envelope.
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
            envelopes,
        };

        printer.out(Paginated {
            inner: envelopes,
            next_page,
        })
    }
}

/// The `envelope list` output, the envelopes of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeListOutput {
    /// The chrono `strftime` format of a date.
    #[serde(skip)]
    pub datetime_fmt: String,
    /// Whether a date is converted to the local timezone first.
    #[serde(skip)]
    pub datetime_local_tz: bool,
    /// Whether a recent date renders relative to today.
    #[serde(skip)]
    pub datetime_relative: bool,
    /// Whether recipients are printed instead of senders.
    #[serde(skip)]
    pub recipient: bool,
    /// The envelopes of this page.
    pub envelopes: Vec<Envelope>,
}

impl fmt::Display for EnvelopeListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let today = Local::now().date_naive();
        let mut records = Records::default();

        for env in &self.envelopes {
            let (label, addresses) = if self.recipient {
                ("To", &env.to)
            } else {
                ("From", &env.from)
            };

            let mut fields = Fields::default();
            fields.push(label, sanitize::text(&format_addresses(addresses)));
            fields.push("Date", self.format_date(env.date, today));
            fields.push("Size", format_size(env.size, BINARY));

            records.push(Record {
                title: sanitize::text(&env.id).into_owned(),
                state: format_state(&env.flags, env.has_attachment),
                fields,
                body: sanitize::text(&env.subject).into_owned(),
            });
        }

        write!(f, "{records}")
    }
}

/// Renders the state words of a message: `unread`, `answered`, `flagged`
/// and `attachment`, in that order, each only when it applies.
///
/// An unknown attachment says nothing.
pub fn format_state(flags: &BTreeSet<Flag>, has_attachment: Option<bool>) -> Vec<String> {
    let mut state = Vec::new();

    if !flags.iter().any(Flag::is_seen) {
        state.push(String::from("unread"));
    }
    if flags.iter().any(Flag::is_answered) {
        state.push(String::from("answered"));
    }
    if flags.iter().any(Flag::is_flagged) {
        state.push(String::from("flagged"));
    }
    if has_attachment == Some(true) {
        state.push(String::from("attachment"));
    }

    state
}

/// Renders addresses as a comma-separated list of `Name <email>`, or the
/// bare email when it has no name.
pub fn format_addresses(addrs: &[Address]) -> String {
    addrs
        .iter()
        .map(|a| match &a.name {
            Some(name) if !name.is_empty() => format!("{name} <{}>", a.email),
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
            envelopes: Vec::new(),
        }
    }

    fn envelope(id: &str, flags: &[IanaFlag], from: Address, subject: &str) -> Envelope {
        Envelope {
            id: String::from(id),
            message_id: None,
            in_reply_to: Vec::new(),
            flags: flags.iter().copied().map(Flag::from_iana).collect(),
            subject: String::from(subject),
            from: vec![from],
            to: Vec::new(),
            date: DateTime::parse_from_rfc3339("2026-11-11T10:00:00+01:00").ok(),
            size: 2048,
            has_attachment: None,
        }
    }

    fn address(name: Option<&str>, email: &str) -> Address {
        Address {
            name: name.map(String::from),
            email: String::from(email),
        }
    }

    /// The list prints as records: the id with its state words, the
    /// sender, date and size, then the subject as the body.
    #[test]
    fn the_list_prints_as_records() {
        let mut envelopes = envelopes(false);
        envelopes.datetime_fmt = String::from("%F %R%:z");

        let alice = address(Some("Alice"), "alice@example.org");
        let mut unread = envelope("42", &[IanaFlag::Flagged], alice, "Lunch on Friday?");
        unread.has_attachment = Some(true);

        let bob = address(None, "bob@example.org");
        let flags = [IanaFlag::Seen, IanaFlag::Answered];
        let mut answered = envelope("41", &flags, bob, "Re: the report");
        answered.has_attachment = Some(false);

        let carol = address(Some("Carol"), "carol@example.org");
        let mut seen = envelope("40", &[IanaFlag::Seen], carol, "");
        seen.date = None;

        envelopes.envelopes = vec![unread, answered, seen];

        let expected = "\
42 (unread, flagged, attachment)
From:  Alice <alice@example.org>
Date:  2026-11-11 10:00+01:00
Size:  2 KiB

    Lunch on Friday?

41 (answered)
From:  bob@example.org
Date:  2026-11-11 10:00+01:00
Size:  2 KiB

    Re: the report

40
From:  Carol <carol@example.org>
Date:
Size:  2 KiB
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
