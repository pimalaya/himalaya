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
use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::imap::{
    client::ImapClient,
    envelope::search::SearchCriteriaArgs,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
    utils::decode_mime,
};

/// Thread IMAP messages (THREAD, RFC 5256).
///
/// Groups messages matching the given criteria into conversation
/// threads using --algorithm. Requires the THREAD extension.
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

/// IMAP THREAD algorithm (RFC 5256).
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum ThreadAlgorithmArg {
    #[default]
    References,
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

fn collect_thread_ids(threads: &[Thread]) -> Vec<NonZeroU32> {
    let mut ids = Vec::new();
    for thread in threads {
        collect_thread_ids_recursive(thread, &mut ids);
    }
    ids
}

fn collect_thread_ids_recursive(thread: &Thread, ids: &mut Vec<NonZeroU32>) {
    match thread {
        Thread::Members { prefix, answers } => {
            // Vec1 can be converted to a slice via as_ref()
            ids.extend(prefix.as_ref().iter().copied());
            if let Some(nested) = answers {
                // Vec2 can also be converted to a slice via as_ref()
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
                    // NString wraps Option<IString>, access via .0
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

/// One flattened THREAD node: a message id with its subject and depth.
#[derive(Clone, Debug, Serialize)]
pub struct ThreadEntry {
    pub id: u32,
    pub subject: String,
    pub depth: usize,
}

/// Renderable tree of THREAD results with their fetched subjects.
pub struct ThreadResultsTable {
    threads: Vec<Thread>,
    subjects: HashMap<u32, String>,
}

impl ThreadResultsTable {
    pub fn new(threads: Vec<Thread>, subjects: HashMap<u32, String>) -> Self {
        Self { threads, subjects }
    }

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
