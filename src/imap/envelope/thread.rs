//! # IMAP thread
//!
//! The `imap thread` command, RFC 5256 `THREAD`.

use io_imap::client::ImapClient as _;
use std::{collections::HashMap, fmt, num::NonZeroU32};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use io_imap::{
    rfc3501::{fetch::ImapMessageFetchOptions, select::ImapMailboxSelectOptions},
    rfc5256::thread::ImapMessageThreadOptions,
    types::{
        extensions::thread::{Thread, ThreadingAlgorithm},
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
        sequence::SequenceSet,
    },
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::imap::{
    client::ImapClient,
    envelope::search::SearchCriteriaArgs,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
    utils::decode_mime,
};

/// Thread messages (THREAD, RFC 5256).
///
/// Groups the matching messages into conversations. The server has to
/// advertise the THREAD extension.
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
    /// the subjects of the messages it returned.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            client.select(mailbox, ImapMailboxSelectOptions::default())?;
        }

        let search_criteria = self.criteria.into_criteria()?;

        let threads = client.thread(
            self.algorithm.into(),
            search_criteria,
            ImapMessageThreadOptions { uid: !self.seq },
        )?;

        let all_ids = collect_thread_ids(&threads);
        let subjects = if !all_ids.is_empty() {
            fetch_subjects(client, &all_ids, !self.seq)?
        } else {
            HashMap::new()
        };

        let table = ThreadResultsTable::new(threads, subjects);

        printer.out(table)
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

/// Fetches the subject of every threaded message, so the tree reads as
/// conversations rather than as bare ids.
fn fetch_subjects(
    client: &mut ImapClient,
    ids: &[NonZeroU32],
    uid: bool,
) -> Result<HashMap<u32, String>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let seq_set_str = ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let sequence_set: SequenceSet = seq_set_str.parse()?;

    let item_names = MacroOrMessageDataItemNames::MessageDataItemNames(vec![
        MessageDataItemName::Envelope,
        MessageDataItemName::Uid,
    ]);

    let data = client.fetch(
        sequence_set,
        item_names,
        ImapMessageFetchOptions {
            uid,
            modifiers: Vec::new(),
        },
    )?;

    let mut subjects: HashMap<u32, String> = HashMap::new();

    for (seq, items) in data {
        let mut id = seq.get();
        let mut subject = String::new();

        for item in items.into_iter() {
            match item {
                MessageDataItem::Uid(uid_val) if uid => {
                    id = uid_val.get();
                }
                MessageDataItem::Envelope(env) => {
                    if let Some(s) = &env.subject.0 {
                        subject = decode_mime(&String::from_utf8_lossy(s.as_ref()));
                    }
                }
                _ => {}
            }
        }

        subjects.insert(id, subject);
    }

    Ok(subjects)
}

/// One flattened thread node.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ThreadEntry {
    /// The UID, or the message number under `--seq`.
    pub id: u32,
    /// The subject fetched for that message.
    pub subject: String,
    /// How deep in its conversation the message sits.
    pub depth: usize,
}

/// The `imap thread` output, a tree of conversations.
pub struct ThreadResultsTable {
    threads: Vec<Thread>,
    subjects: HashMap<u32, String>,
}

impl ThreadResultsTable {
    /// Pairs the threads the server returned with the fetched subjects.
    pub fn new(threads: Vec<Thread>, subjects: HashMap<u32, String>) -> Self {
        Self { threads, subjects }
    }

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
                    let id_val: u32 = id.get();
                    let subject = self.subjects.get(&id_val).cloned().unwrap_or_default();
                    entries.push(ThreadEntry {
                        id: id_val,
                        subject,
                        depth: depth + i,
                    });
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
}

impl fmt::Display for ThreadResultsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.threads.is_empty() {
            writeln!(f)?;
            writeln!(f, "No threads found")?;
            return Ok(());
        }

        let mut thread_num = 0;

        writeln!(f)?;

        for thread in &self.threads {
            thread_num += 1;
            writeln!(f, "Thread {thread_num}:")?;
            self.display_thread(f, thread, 1)?;
            writeln!(f)?;
        }

        writeln!(f, "Found {} thread(s)", self.threads.len())?;
        Ok(())
    }
}

impl ThreadResultsTable {
    fn display_thread(
        &self,
        f: &mut fmt::Formatter<'_>,
        thread: &Thread,
        depth: usize,
    ) -> fmt::Result {
        let indent = "  ".repeat(depth);

        match thread {
            Thread::Members { prefix, answers } => {
                for (i, id) in prefix.as_ref().iter().enumerate() {
                    let id_val: u32 = id.get();
                    let subject = self.subjects.get(&id_val).cloned().unwrap_or_default();
                    let connector = if i == 0 && depth > 0 {
                        "\u{2514}\u{2500}"
                    } else {
                        "  "
                    };
                    writeln!(f, "{indent}{connector} {id_val}: {subject}")?;
                }
                if let Some(nested) = answers {
                    for t in nested.as_ref().iter() {
                        self.display_thread(f, t, depth + 1)?;
                    }
                }
            }
            Thread::Nested { answers } => {
                for t in answers.as_ref().iter() {
                    self.display_thread(f, t, depth)?;
                }
            }
        }

        Ok(())
    }
}

impl Serialize for ThreadResultsTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("ThreadResultsTable", 1)?;
        s.serialize_field("threads", &self.build_entries())?;
        s.end()
    }
}

/// Mirrors the JSON shape [`ThreadResultsTable`]'s hand-written
/// [`Serialize`] produces, only so the schema can be derived from it.
#[derive(JsonSchema)]
#[allow(dead_code)]
struct ThreadResultsTableSchema {
    threads: Vec<ThreadEntry>,
}

impl JsonSchema for ThreadResultsTable {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        <ThreadResultsTableSchema as JsonSchema>::schema_name()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        <ThreadResultsTableSchema as JsonSchema>::json_schema(generator)
    }
}
