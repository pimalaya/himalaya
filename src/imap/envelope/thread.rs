//! # IMAP thread
//!
//! The `imap thread` command, RFC 5256 `THREAD`.

use io_imap::client::ImapClient as _;
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    fmt,
    num::NonZeroU32,
};

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use clap::{Parser, ValueEnum};
use io_imap::{
    rfc3501::{fetch::ImapMessageFetchOptions, select::ImapMailboxSelectOptions},
    rfc5256::thread::ImapMessageThreadOptions,
    types::{
        extensions::thread::{Thread, ThreadingAlgorithm},
        fetch::{MacroOrMessageDataItemNames, MessageDataItemName},
        sequence::SequenceSet,
    },
};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::{
    account::context::Account,
    backend::Backend,
    capability::missing,
    email::{address::Address, envelope::Envelope, flag::Flag},
    imap::{
        backend::envelope_from,
        client::ImapClient,
        envelope::search::SearchCriteriaArgs,
        mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
        outcome,
    },
    listing::Listing,
    shared::envelope::list::{format_date, format_flags, format_names},
};

/// Thread messages (THREAD, RFC 5256).
///
/// Groups the matching messages into conversations. The server has to
/// advertise the THREAD extension with the algorithm, or the command is
/// refused before anything is sent.
#[derive(Debug, Parser)]
pub struct ImapEnvelopeThreadCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,
    /// Threading algorithm.
    #[arg(
        short = 'A',
        long,
        value_name = "ALGORITHM",
        default_value = "references"
    )]
    pub algorithm: ThreadAlgorithmArg,
    #[command(flatten)]
    pub criteria: SearchCriteriaArgs,
    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapEnvelopeThreadCommand {
    /// Selects the mailbox unless told not to, threads it, then fetches
    /// the envelopes of the messages it returned.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        let selected = (!self.mailbox_no_select.inner).then(|| self.mailbox_name.inner.clone());
        let mailbox = self.mailbox_name.inner.try_into()?;
        let algorithm: ThreadingAlgorithm = self.algorithm.into();

        // NOTE: a server without the extension answers with a bare parse
        // error (iCloud's `BAD Parse Error`), so the refusal is local.
        if !client.supports_thread(&algorithm) {
            let algorithm = algorithm.to_string().to_ascii_uppercase();
            return Err(missing(format!(
                "The IMAP server does not advertise `THREAD={algorithm}` (RFC 5256)"
            )));
        }

        let result = (|| -> Result<_> {
            if !self.mailbox_no_select.inner {
                client.select(mailbox, ImapMailboxSelectOptions::default())?;
            }

            let search_criteria = self.criteria.into_criteria()?;

            let threads = client.thread(
                algorithm,
                search_criteria,
                ImapMessageThreadOptions { uid: !self.seq },
            )?;

            let ids = collect_thread_ids(&threads);
            let envelopes = fetch_envelopes(client, &ids, !self.seq)?;

            Ok(ImapThreadOutput {
                account: account.name().to_string(),
                mailbox: selected,
                datetime_fmt: account.datetime_fmt().to_string(),
                datetime_local_tz: account.datetime_local_tz(),
                threads,
                envelopes,
            })
        })();

        outcome::print(printer, result)
    }
}

/// The algorithm a `THREAD` groups by, per RFC 5256.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum ThreadAlgorithmArg {
    /// Follow the `References` and `In-Reply-To` headers.
    #[default]
    References,
    /// Group by subject, then order by date.
    OrderedSubject,
}

impl From<ThreadAlgorithmArg> for ThreadingAlgorithm<'static> {
    fn from(arg: ThreadAlgorithmArg) -> Self {
        match arg {
            ThreadAlgorithmArg::References => ThreadingAlgorithm::References,
            ThreadAlgorithmArg::OrderedSubject => ThreadingAlgorithm::OrderedSubject,
        }
    }
}

/// Flattens every thread into the message ids it holds.
fn collect_thread_ids(threads: &[Thread]) -> Vec<NonZeroU32> {
    let mut ids = Vec::new();
    for thread in threads {
        collect_thread_ids_recursive(thread, &mut ids);
    }
    ids
}

/// Walks one thread node, collecting its id and its children's.
fn collect_thread_ids_recursive(thread: &Thread, ids: &mut Vec<NonZeroU32>) {
    match thread {
        Thread::Members { prefix, answers } => {
            ids.extend(prefix.as_ref().iter().copied());
            if let Some(nested) = answers {
                for t in nested.as_ref().iter() {
                    collect_thread_ids_recursive(t, ids);
                }
            }
        }
        Thread::Nested { answers } => {
            for t in answers.as_ref().iter() {
                collect_thread_ids_recursive(t, ids);
            }
        }
    }
}

/// Fetches the envelope of every threaded message, keyed by the id the
/// threads name it by.
fn fetch_envelopes(
    client: &mut ImapClient,
    ids: &[NonZeroU32],
    uid: bool,
) -> Result<HashMap<u32, Envelope>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let sequence_set: SequenceSet = ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
        .parse()?;

    // Without UID, an envelope falls back to its message number as id.
    let mut names = vec![MessageDataItemName::Flags, MessageDataItemName::Envelope];
    if uid {
        names.push(MessageDataItemName::Uid);
    }

    let data = client.fetch(
        sequence_set,
        MacroOrMessageDataItemNames::MessageDataItemNames(names),
        ImapMessageFetchOptions {
            uid,
            modifiers: Vec::new(),
        },
    )?;

    let envelopes = data
        .into_iter()
        .map(|(seq, items)| envelope_from(seq.get(), items.into_inner()))
        .filter_map(|envelope| Some((envelope.id.parse().ok()?, envelope)))
        .collect();

    Ok(envelopes)
}

/// One flattened thread node.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThreadEntry {
    /// The UID, or the message number under `--seq`.
    pub id: u32,
    /// The `Subject:` header.
    pub subject: String,
    /// How deep in its conversation the message sits.
    pub depth: usize,
    /// The senders.
    pub from: Vec<Address>,
    /// The `Date:` header, `null` when it is missing or unparseable.
    pub date: Option<DateTime<FixedOffset>>,
    /// The flags set on the message.
    pub flags: BTreeSet<Flag>,
}

/// The `imap thread` output, a tree of conversations.
pub struct ImapThreadOutput {
    /// The name of the account listed.
    account: String,
    /// The mailbox selected, `None` under `--no-select`.
    mailbox: Option<String>,
    /// The chrono `strftime` format of a date.
    datetime_fmt: String,
    /// Whether a date is converted to the local timezone first.
    datetime_local_tz: bool,
    /// The threads, as the server returned them.
    threads: Vec<Thread>,
    /// The envelope of each threaded message, by id.
    envelopes: HashMap<u32, Envelope>,
}

impl ImapThreadOutput {
    /// Flattens the tree into one entry per message, depth included.
    fn build_entries(&self) -> Vec<ThreadEntry> {
        let mut entries = Vec::new();
        for thread in &self.threads {
            self.build_entries_recursive(thread, 0, &mut entries);
        }
        entries
    }

    fn build_entries_recursive(
        &self,
        thread: &Thread,
        depth: usize,
        entries: &mut Vec<ThreadEntry>,
    ) {
        match thread {
            Thread::Members { prefix, answers } => {
                for (i, id) in prefix.as_ref().iter().enumerate() {
                    entries.push(self.entry(id.get(), depth + i));
                }
                if let Some(nested) = answers {
                    for t in nested.as_ref().iter() {
                        self.build_entries_recursive(t, depth + prefix.as_ref().len(), entries);
                    }
                }
            }
            Thread::Nested { answers } => {
                for t in answers.as_ref().iter() {
                    self.build_entries_recursive(t, depth, entries);
                }
            }
        }
    }

    /// The entry of one message, empty but for its id when its envelope
    /// could not be fetched.
    fn entry(&self, id: u32, depth: usize) -> ThreadEntry {
        let envelope = self.envelopes.get(&id);

        ThreadEntry {
            id,
            subject: envelope.map(|e| e.subject.clone()).unwrap_or_default(),
            depth,
            from: envelope.map(|e| e.from.clone()).unwrap_or_default(),
            date: envelope.and_then(|e| e.date),
            flags: envelope.map(|e| e.flags.clone()).unwrap_or_default(),
        }
    }
}

impl fmt::Display for ImapThreadOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Imap));

        if let Some(mailbox) = &self.mailbox {
            listing.head("Mailbox", mailbox);
        }

        for entry in self.build_entries() {
            // A message whose envelope could not be fetched shows no flag.
            let flags = match self.envelopes.contains_key(&entry.id) {
                true => format_flags(&entry.flags, None),
                false => String::from("    "),
            };

            let date = format_date(entry.date, &self.datetime_fmt, self.datetime_local_tz);
            let indent = "  ".repeat(entry.depth);
            let subject = sanitize::text(&entry.subject);

            listing.items.push([
                entry.id.to_string(),
                flags,
                date,
                sanitize::text(&format_names(&entry.from)).into_owned(),
                format!("{indent}{subject}"),
            ]);
        }

        write!(f, "{listing}")
    }
}

impl Serialize for ImapThreadOutput {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("ImapThreadOutput", 1)?;
        s.serialize_field("threads", &self.build_entries())?;
        s.end()
    }
}

/// Mirrors the JSON shape [`ImapThreadOutput`]'s hand-written
/// [`Serialize`] produces, only so the schema can be derived from it.
#[derive(JsonSchema)]
#[schemars(rename = "ImapThreadOutput")]
#[allow(dead_code)]
struct ImapThreadOutputSchema {
    threads: Vec<ThreadEntry>,
}

impl JsonSchema for ImapThreadOutput {
    fn schema_name() -> Cow<'static, str> {
        <ImapThreadOutputSchema as JsonSchema>::schema_name()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        <ImapThreadOutputSchema as JsonSchema>::json_schema(generator)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, num::NonZeroU32};

    use chrono::DateTime;
    use io_imap::types::{
        core::{Vec1, Vec2},
        extensions::thread::Thread,
    };

    use crate::{
        email::{
            address::Address,
            envelope::Envelope,
            flag::{Flag, IanaFlag},
        },
        imap::envelope::thread::ImapThreadOutput,
    };

    fn members(ids: &[u32], answers: Vec<Thread>) -> Thread {
        let prefix: Vec<_> = ids.iter().filter_map(|id| NonZeroU32::new(*id)).collect();

        Thread::Members {
            prefix: Vec1::try_from(prefix).unwrap(),
            answers: Vec2::try_from(answers).ok(),
        }
    }

    fn envelope(id: u32, seen: bool, name: &str, subject: &str) -> (u32, Envelope) {
        let flags = match seen {
            true => [Flag::from_iana(IanaFlag::Seen)].into(),
            false => Default::default(),
        };

        let envelope = Envelope {
            id: id.to_string(),
            message_id: None,
            in_reply_to: Vec::new(),
            flags,
            subject: String::from(subject),
            from: vec![Address {
                name: Some(String::from(name)),
                email: format!("{}@example.org", name.to_lowercase()),
            }],
            to: Vec::new(),
            date: DateTime::parse_from_rfc3339("2026-11-11T10:00:00+01:00").ok(),
            size: 0,
            has_attachment: None,
        };

        (id, envelope)
    }

    /// A reply follows the message it answers, its subject one step further
    /// in: a run of members nests each under the one before, and the
    /// answers of a message the mailbox lacks start back at the margin.
    #[test]
    fn replies_follow_what_they_answer() {
        let lunch = members(&[1], vec![members(&[2, 4], vec![]), members(&[3], vec![])]);
        let orphans = Thread::Nested {
            answers: Vec2::try_from(vec![members(&[5], vec![]), members(&[6], vec![])]).unwrap(),
        };

        let output = ImapThreadOutput {
            account: String::from("work"),
            mailbox: Some(String::from("INBOX")),
            datetime_fmt: String::from("%F %R"),
            datetime_local_tz: false,
            threads: vec![lunch, orphans],
            envelopes: HashMap::from([
                envelope(1, true, "Me", "Lunch"),
                envelope(2, false, "Alice", "Re: Lunch"),
                envelope(3, true, "Bob", "Re: Lunch"),
                envelope(4, false, "Carol", "Re: Lunch"),
                envelope(5, true, "Bob", "Re: Invoice"),
                envelope(6, true, "Me", "Re: Invoice"),
            ]),
        };

        let expected = "\
Account:    work (imap)
Mailbox:    INBOX

1        2026-11-11 10:00  Me     Lunch
2  *     2026-11-11 10:00  Alice    Re: Lunch
4  *     2026-11-11 10:00  Carol      Re: Lunch
3        2026-11-11 10:00  Bob      Re: Lunch
5        2026-11-11 10:00  Bob    Re: Invoice
6        2026-11-11 10:00  Me     Re: Invoice
";
        assert_eq!(output.to_string(), expected);
    }
}
