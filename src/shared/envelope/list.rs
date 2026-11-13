//! # Envelope list
//!
//! The `envelope list` command, listing one page of a mailbox.

use std::{collections::BTreeSet, fmt};

use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset, Local};
use clap::Parser;
use pimalaya_cli::{clap::args::PageTokenArg, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    email::{address::Address, envelope::Envelope, flag::Flag},
    listing::{Listing, ListingPage},
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
    #[arg(value_parser = clap::value_parser!(u32).range(1..))]
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
                .ok()
                .filter(|page: &u32| *page > 0)
                .with_context(|| format!("Invalid page token {token}"))?,
            None => self.page,
        };
        let page = Some(page);
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

        let resolution = client.resolution();

        printer.out(EnvelopeListOutput {
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
            account: resolution.account.clone(),
            backend: resolution.storage,
            mailbox,
            query: None,
            page,
            page_size,
            recipient: self.recipient,
            envelopes,
            next_page,
        })
    }
}

/// The `envelope list` output, the envelopes of one page and the cursor
/// of the next.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeListOutput {
    /// The chrono `strftime` format of a date.
    #[serde(skip)]
    pub datetime_fmt: String,
    /// Whether a date is converted to the local timezone first.
    #[serde(skip)]
    pub datetime_local_tz: bool,
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The storage backend the account resolved to.
    #[serde(skip)]
    pub backend: Option<Backend>,
    /// The mailbox listed, its alias resolved.
    #[serde(skip)]
    pub mailbox: String,
    /// The search query as given, `None` for a plain listing.
    #[serde(skip)]
    pub query: Option<String>,
    /// The page listed, `None` when the listing is not paged.
    #[serde(skip)]
    pub page: Option<u32>,
    /// The size of a page, `None` when the listing is not paged.
    #[serde(skip)]
    pub page_size: Option<u32>,
    /// Whether recipients are printed instead of senders.
    #[serde(skip)]
    pub recipient: bool,
    /// The envelopes of this page.
    pub envelopes: Vec<Envelope>,
    /// The cursor of the next page, `null` once the listing is complete,
    /// given back through `--page-token`.
    pub next_page: Option<String>,
}

impl fmt::Display for EnvelopeListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, self.backend);
        listing.head("Mailbox", &self.mailbox);

        if let Some(query) = &self.query {
            listing.head("Query", query);
        }

        for env in &self.envelopes {
            let fmt = &self.datetime_fmt;
            let [id, flags, date, mut names, subject] =
                envelope_cells(env, fmt, self.datetime_local_tz);

            if self.recipient {
                names = match env.to.is_empty() {
                    true => String::new(),
                    false => sanitize::text(&format!("to {}", format_names(&env.to))).into_owned(),
                };
            }

            listing.items.push([id, flags, date, names, subject]);
        }

        let page = match (self.page, self.page_size) {
            (Some(page), Some(size)) => ListingPage::Numbered {
                page: page.into(),
                size: size.into(),
            },
            _ => ListingPage::All,
        };

        let count = self.envelopes.len();
        listing.page(
            page,
            count,
            ["envelope", "envelopes"],
            self.next_page.as_deref(),
        );

        write!(f, "{listing}")
    }
}

/// The cells of an envelope's line, sanitized: its id, its flags, its date,
/// its senders' names and its subject.
pub fn envelope_cells(env: &Envelope, datetime_fmt: &str, local_tz: bool) -> [String; 5] {
    [
        sanitize::text(&env.id).into_owned(),
        format_flags(&env.flags, env.has_attachment),
        format_date(env.date, datetime_fmt, local_tz),
        sanitize::text(&format_names(&env.from)).into_owned(),
        sanitize::text(&env.subject).into_owned(),
    ]
}

/// Renders the flags of a message as four fixed columns: `*` when unread,
/// `R` when answered, `!` when flagged and `@` when it has an attachment,
/// a blank otherwise.
///
/// An unknown attachment is a blank.
pub fn format_flags(flags: &BTreeSet<Flag>, has_attachment: Option<bool>) -> String {
    let marks = [
        (!flags.iter().any(Flag::is_seen), '*'),
        (flags.iter().any(Flag::is_answered), 'R'),
        (flags.iter().any(Flag::is_flagged), '!'),
        (has_attachment == Some(true), '@'),
    ];

    marks
        .into_iter()
        .map(|(on, mark)| if on { mark } else { ' ' })
        .collect()
}

/// Renders the display names of addresses, comma-separated, the email
/// standing for a nameless one.
pub fn format_names(addrs: &[Address]) -> String {
    addrs
        .iter()
        .map(|a| match &a.name {
            Some(name) if !name.is_empty() => name.as_str(),
            _ => a.email.as_str(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Renders addresses as a comma-separated list of `Name <email>`, or the
/// bare email when it has no name.
#[cfg(feature = "imap")]
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

/// Renders a date with a chrono `strftime` format, in the local timezone
/// when asked for, nothing without one.
pub fn format_date(date: Option<DateTime<FixedOffset>>, fmt: &str, local_tz: bool) -> String {
    match date {
        None => String::new(),
        Some(date) if local_tz => date.with_timezone(&Local).format(fmt).to_string(),
        Some(date) => date.format(fmt).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use crate::{
        backend::Backend,
        email::{
            address::Address,
            envelope::Envelope,
            flag::{Flag, IanaFlag},
        },
        shared::envelope::list::EnvelopeListOutput,
    };

    fn envelopes() -> EnvelopeListOutput {
        EnvelopeListOutput {
            datetime_fmt: String::from("%F %R"),
            datetime_local_tz: false,
            account: String::from("work"),
            backend: Some(Backend::Imap),
            mailbox: String::from("INBOX"),
            query: None,
            page: Some(1),
            page_size: Some(25),
            recipient: false,
            envelopes: Vec::new(),
            next_page: None,
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

    /// One aligned line per envelope: the id, the flags, the date, the
    /// sender's name, then the subject, `-` standing for an empty value,
    /// under the account, the mailbox and the page.
    #[test]
    fn an_envelope_prints_on_one_line() {
        let mut envelopes = envelopes();

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
Account:    work (imap)
Mailbox:    INBOX
Page:       1, envelopes 1 to 3, the last

42  * !@  2026-11-11 10:00  Alice            Lunch on Friday?
41   R    2026-11-11 10:00  bob@example.org  Re: the report
40        -                 Carol            -
";
        assert_eq!(envelopes.to_string(), expected);
    }

    /// The header names the envelopes the page holds, then the flag
    /// reaching the next page while one follows, its values in one column.
    #[test]
    fn the_header_tells_where_the_page_stands() {
        let mut envelopes = envelopes();
        envelopes.page = Some(2);
        envelopes.page_size = Some(2);
        envelopes.next_page = Some(String::from("3"));

        let alice = address(Some("Alice"), "alice@example.org");
        envelopes.envelopes = vec![
            envelope("4", &[IanaFlag::Seen], alice.clone(), "Hi"),
            envelope("3", &[IanaFlag::Seen], alice, "Re: Hi"),
        ];

        let expected = "\
Account:    work (imap)
Mailbox:    INBOX
Page:       2, envelopes 3 to 4
Next page:  --page 3

4        2026-11-11 10:00  Alice  Hi
3        2026-11-11 10:00  Alice  Re: Hi
";
        assert_eq!(envelopes.to_string(), expected);

        envelopes.page = Some(3);
        envelopes.next_page = None;
        envelopes.envelopes.truncate(1);

        let expected = "\
Account:    work (imap)
Mailbox:    INBOX
Page:       3, envelope 5, the last

4        2026-11-11 10:00  Alice  Hi
";
        assert_eq!(envelopes.to_string(), expected);

        envelopes.page = Some(4);
        envelopes.envelopes.clear();

        let expected = "\
Account:    work (imap)
Mailbox:    INBOX
Page:       4, empty
";
        assert_eq!(envelopes.to_string(), expected);
    }

    /// A search names its query under the mailbox.
    #[test]
    fn a_search_names_its_query() {
        let mut envelopes = envelopes();
        envelopes.query = Some(String::from("from alice order by date asc"));

        let expected = "\
Account:    work (imap)
Mailbox:    INBOX
Query:      from alice order by date asc
Page:       1, empty
";
        assert_eq!(envelopes.to_string(), expected);
    }

    /// Under `--recipient`, the addresses read as recipients, and an
    /// envelope without any prints none.
    #[test]
    fn recipients_print_after_to() {
        let mut envelopes = envelopes();
        envelopes.recipient = true;

        let me = address(None, "me@example.org");
        let mut sent = envelope("2", &[IanaFlag::Seen], me.clone(), "Hi");
        sent.to = vec![address(Some("Bob"), "bob@example.org")];
        let mut draft = envelope("1", &[IanaFlag::Seen], me, "Notes");
        draft.date = None;

        envelopes.envelopes = vec![sent, draft];

        let expected = "\
Account:    work (imap)
Mailbox:    INBOX
Page:       1, envelopes 1 to 2, the last

2        2026-11-11 10:00  to Bob  Hi
1        -                 -       Notes
";
        assert_eq!(envelopes.to_string(), expected);
    }

    #[test]
    fn control_characters_from_the_sender_are_not_printed() {
        let mut envelopes = envelopes();
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
}
