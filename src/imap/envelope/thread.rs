use std::{collections::HashMap, fmt, num::NonZeroU32};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{fetch::*, select::*, thread::*},
    types::{
        extensions::thread::{Thread, ThreadingAlgorithm},
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
        sequence::SequenceSet,
    },
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::{Serialize, Serializer};

use crate::imap::{
    account::ImapAccount,
    envelope::{list::decode_mime, search::parse_query},
    mailbox::arg::MailboxNameOptionalArg,
    stream,
};

/// Thread messages by algorithm.
///
/// This command groups messages into conversation threads using the
/// specified threading algorithm. Requires the THREAD IMAP extension.
///
/// Threading algorithms:
///   - references (default) - uses References and In-Reply-To headers
///   - orderedsubject       - groups by normalized subject
#[derive(Debug, Parser)]
pub struct ThreadEnvelopesCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalArg,

    /// Threading algorithm (orderedsubject or references).
    #[arg(short = 'A', long, default_value = "references")]
    pub algorithm: String,

    /// Search query (same syntax as search command).
    #[arg(name = "query", value_name = "QUERY", default_value = "all")]
    pub query: String,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ThreadEnvelopesCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let mailbox = self.mailbox.name.try_into()?;

        // SELECT mailbox
        let mut arg = None;
        let mut coroutine = ImapSelect::new(context, mailbox);

        let context = loop {
            match coroutine.resume(arg.take()) {
                ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapSelectResult::Ok { context, .. } => break context,
                ImapSelectResult::Err { err, .. } => bail!(err),
            }
        };

        // Parse threading algorithm
        let algorithm = parse_algorithm(&self.algorithm)?;

        // Parse search criteria
        let search_criteria = parse_query(&self.query)?;

        // THREAD
        let mut arg = None;
        let mut coroutine = ImapThread::new(context, algorithm, search_criteria, !self.seq);

        let (context, threads) = loop {
            match coroutine.resume(arg.take()) {
                ImapThreadResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapThreadResult::Ok {
                    context, threads, ..
                } => break (context, threads),
                ImapThreadResult::Err { err, .. } => bail!(err),
            }
        };

        // Collect all message IDs from threads to fetch subjects
        let all_ids = collect_thread_ids(&threads);

        // Fetch subjects for all messages in threads
        let subjects = if !all_ids.is_empty() {
            fetch_subjects(&mut stream, context, &all_ids, !self.seq)?
        } else {
            HashMap::new()
        };

        let table = ThreadResultsTable::new(threads, subjects, !self.seq);

        printer.out(table)?;
        Ok(())
    }
}

fn parse_algorithm(s: &str) -> Result<ThreadingAlgorithm<'static>> {
    match s.to_lowercase().as_str() {
        "references" => Ok(ThreadingAlgorithm::References),
        "orderedsubject" => Ok(ThreadingAlgorithm::OrderedSubject),
        _ => bail!("Unknown threading algorithm: {s}. Valid options: references, orderedsubject"),
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
    stream: &mut stream::Stream,
    context: io_imap::context::ImapContext,
    ids: &[NonZeroU32],
    uid: bool,
) -> Result<HashMap<u32, String>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    // Build sequence set from IDs
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

    let mut arg = None;
    let mut coroutine = ImapFetch::new(context, sequence_set, item_names, uid);

    let data = loop {
        match coroutine.resume(arg.take()) {
            ImapFetchResult::Io { io } => arg = Some(handle(&mut *stream, io)?),
            ImapFetchResult::Ok { data, .. } => break data,
            ImapFetchResult::Err { err, .. } => bail!(err),
        }
    };

    let mut subjects: HashMap<u32, String> = HashMap::new();

    for (seq, items) in data {
        let mut id = seq.get();
        let mut subject = String::new();

        for item in items.into_iter() {
            match item {
                MessageDataItem::Uid(uid_val) => {
                    if uid {
                        id = uid_val.get();
                    }
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

#[derive(Clone, Debug, Serialize)]
pub struct ThreadEntry {
    pub id: u32,
    pub subject: String,
    pub depth: usize,
}

pub struct ThreadResultsTable {
    threads: Vec<Thread>,
    subjects: HashMap<u32, String>,
    #[allow(dead_code)]
    uid_mode: bool,
}

impl ThreadResultsTable {
    pub fn new(threads: Vec<Thread>, subjects: HashMap<u32, String>, uid_mode: bool) -> Self {
        Self {
            threads,
            subjects,
            uid_mode,
        }
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
        self.build_entries().serialize(serializer)
    }
}
