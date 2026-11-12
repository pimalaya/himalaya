//! # Envelope list
//!
//! The `envelope list` command, listing one page of a mailbox.

use std::{borrow::Cow, collections::BTreeSet, fmt};

use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset, Local};
use clap::Parser;
use pimalaya_cli::{
    clap::args::PageTokenArg,
    output::Paginated,
    printer::Printer,
    sanitize,
    tree::{Node, Trees},
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
            mailbox,
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
    /// The mailbox listed, its alias resolved, the root of the text.
    #[serde(skip)]
    pub mailbox: String,
    /// Whether recipients are printed instead of senders.
    #[serde(skip)]
    pub recipient: bool,
    /// The envelopes of this page.
    pub envelopes: Vec<Envelope>,
}

impl fmt::Display for EnvelopeListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut mailbox = Node {
            title: sanitize::text(&self.mailbox).into_owned(),
            ..Node::default()
        };

        for env in &self.envelopes {
            let addresses = match self.recipient {
                true if env.to.is_empty() => String::new(),
                true => format!("to {}", format_addresses(&env.to)),
                false => format_addresses(&env.from),
            };

            let date = format_date(env.date, &self.datetime_fmt, self.datetime_local_tz);
            let state = format_state(&env.flags, env.has_attachment);

            mailbox.children.push(Node {
                title: sanitize::text(&env.subject).into_owned(),
                detail: format_detail(&date, &env.id, &state, &[&addresses]),
                children: Vec::new(),
            });
        }

        let trees = Trees {
            items: vec![mailbox],
        };

        write!(f, "{trees}")
    }
}

/// Renders the line under an envelope's subject: its date, its id with
/// its state words in parentheses, then the rest (its addresses), two
/// spaces apart, each only when it holds something.
pub fn format_detail(date: &str, id: &str, state: &[String], rest: &[&str]) -> String {
    let mut id = sanitize::text(id).into_owned();

    if !state.is_empty() {
        id = format!("{id} ({})", state.join(", "));
    }

    let rest = rest.iter().map(|part| sanitize::text(part));

    [Cow::Borrowed(date), Cow::Owned(id)]
        .into_iter()
        .chain(rest)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("  ")
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
        email::{
            address::Address,
            envelope::Envelope,
            flag::{Flag, IanaFlag},
        },
        shared::envelope::list::EnvelopeListOutput,
    };

    fn envelopes() -> EnvelopeListOutput {
        EnvelopeListOutput {
            datetime_fmt: String::from("%F %R%:z"),
            datetime_local_tz: false,
            mailbox: String::from("INBOX"),
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

    /// The envelopes hang from their mailbox: the subject on the branch,
    /// then the date, the id with its state words and the sender.
    #[test]
    fn the_list_hangs_from_its_mailbox() {
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
INBOX
├─ Lunch on Friday?
│  2026-11-11 10:00+01:00  42 (unread, flagged, attachment)  Alice <alice@example.org>
├─ Re: the report
│  2026-11-11 10:00+01:00  41 (answered)  bob@example.org
└─
   40  Carol <carol@example.org>
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
INBOX
├─ Hi
│  2026-11-11 10:00+01:00  2  to Bob <bob@example.org>
└─ Notes
   1
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
