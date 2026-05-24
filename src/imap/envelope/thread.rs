// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{collections::HashMap, fmt, num::NonZeroU32};

use anyhow::{Result, bail};
use clap::Parser;
use io_imap::types::{
    extensions::thread::{Thread, ThreadingAlgorithm},
    fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
    sequence::SequenceSet,
};
use pimalaya_cli::printer::Printer;
use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::imap::{
    client::ImapClient,
    envelope::{list::decode_mime, search::parse_query},
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
};

/// Thread IMAP messages by algorithm.
///
/// This command groups messages into conversation threads using the
/// specified threading algorithm. Requires the THREAD IMAP extension.
///
/// Threading algorithms:
///   - references (default) - uses References and In-Reply-To headers
///   - orderedsubject       - groups by normalized subject
#[derive(Debug, Parser)]
pub struct ImapEnvelopeThreadCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

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

impl ImapEnvelopeThreadCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            client.select(mailbox)?;
        }

        let algorithm = parse_algorithm(&self.algorithm)?;
        let search_criteria = parse_query(&self.query)?;

        let threads = client.thread(algorithm, search_criteria, !self.seq)?;

        let all_ids = collect_thread_ids(&threads);
        let subjects = if !all_ids.is_empty() {
            fetch_subjects(&mut client, &all_ids, !self.seq)?
        } else {
            HashMap::new()
        };

        let table = ThreadResultsTable::new(threads, subjects);

        printer.out(table)
    }
}

fn parse_algorithm(s: &str) -> Result<ThreadingAlgorithm<'static>> {
    match s.to_lowercase().as_str() {
        "references" => Ok(ThreadingAlgorithm::References),
        "orderedsubject" => Ok(ThreadingAlgorithm::OrderedSubject),
        _ => bail!("Unknown threading algorithm `{s}`, valid options: references, orderedsubject"),
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

    let data = client.fetch(sequence_set, item_names, uid)?;

    let mut subjects: HashMap<u32, String> = HashMap::new();

    for (seq, items) in data {
        let mut id = seq.get();
        let mut subject = String::new();

        for item in items.into_iter() {
            match item {
                MessageDataItem::Uid(uid_val)
                    if uid => {
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

#[derive(Clone, Debug, Serialize)]
pub struct ThreadEntry {
    pub id: u32,
    pub subject: String,
    pub depth: usize,
}

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
