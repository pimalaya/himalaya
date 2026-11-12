//! # pimdir thread
//!
//! The `pimdir thread` command, listing the thread of one message through
//! the store's search index.

use std::{collections::HashMap, fmt};

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{
    printer::Printer,
    sanitize,
    tree::{Node, Trees},
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    pimdir::{
        client::PimdirClient,
        index::{PimdirListStyle, PimdirMailThread},
    },
    shared::envelope::list::{format_addresses, format_detail, format_state},
};

/// List the thread of a message, oldest first.
///
/// The thread is read from the store's search index, which joins a reply
/// to its parent by `In-Reply-To`, else `References`, never by subject. A
/// message no thread holds is listed alone. The text hangs each message
/// under the earlier one its `In-Reply-To` names; JSON lists them flat.
#[derive(Debug, Parser)]
pub struct PimdirThreadCommand {
    /// The message id, as a listing prints it.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl PimdirThreadCommand {
    /// Reads the thread and prints its messages.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let thread = client.mail_thread(&self.id)?;
        printer.out(PimdirThreadOutput {
            style: PimdirListStyle::new(account),
            thread,
        })
    }
}

/// The `pimdir thread` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirThreadOutput {
    /// How the list renders.
    #[serde(skip)]
    pub style: PimdirListStyle,
    /// The thread, every message of it in the account, oldest first.
    #[serde(flatten)]
    pub thread: PimdirMailThread,
}

impl fmt::Display for PimdirThreadOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let trees = Trees {
            items: self.nodes(),
        };

        write!(f, "{trees}")
    }
}

impl PimdirThreadOutput {
    /// The thread as trees: each message under the earlier one its
    /// `In-Reply-To` names, a message whose parent the account lacks at
    /// the top.
    fn nodes(&self) -> Vec<Node> {
        let hits = &self.thread.hits;
        let mut children = vec![Vec::new(); hits.len()];
        let mut roots = Vec::new();
        let mut earlier: HashMap<&str, usize> = HashMap::new();

        for (i, hit) in hits.iter().enumerate() {
            // Only an earlier message is a parent, so no cycle forms.
            let parent = hit
                .envelope
                .in_reply_to
                .iter()
                .find_map(|id| earlier.get(id.as_str()).copied());

            match parent {
                Some(parent) => children[parent].push(i),
                None => roots.push(i),
            }

            if let Some(id) = &hit.envelope.message_id {
                earlier.entry(id.as_str()).or_insert(i);
            }
        }

        roots.into_iter().map(|i| self.node(i, &children)).collect()
    }

    /// The node of one message: its subject on the branch, then its date,
    /// its id with its state words, its mailboxes and its senders.
    fn node(&self, i: usize, children: &[Vec<usize>]) -> Node {
        let hit = &self.thread.hits[i];
        let envelope = &hit.envelope;

        let date = self.style.date(envelope.date);
        let state = format_state(&envelope.flags, envelope.has_attachment);
        let from = format_addresses(&envelope.from);
        let mailboxes = match hit.mailboxes.is_empty() {
            true => String::new(),
            false => format!("in {}", hit.mailboxes.join(", ")),
        };

        Node {
            title: sanitize::text(&envelope.subject).into_owned(),
            detail: format_detail(&date, &envelope.id, &state, &[&mailboxes, &from]),
            children: children[i]
                .iter()
                .map(|&child| self.node(child, children))
                .collect(),
        }
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
        pimdir::{
            index::{PimdirListStyle, PimdirMailHit, PimdirMailThread},
            thread::PimdirThreadOutput,
        },
    };

    fn hit(
        id: &str,
        parent: Option<&str>,
        seen: bool,
        name: &str,
        mailboxes: &[&str],
    ) -> PimdirMailHit {
        let flags = match seen {
            true => [Flag::from_iana(IanaFlag::Seen)].into(),
            false => Default::default(),
        };

        let subject = match parent {
            Some(_) => "Re: Lunch",
            None => "Lunch",
        };

        PimdirMailHit {
            envelope: Envelope {
                id: String::from(id),
                message_id: Some(format!("{id}@example.org")),
                in_reply_to: parent
                    .map(|p| format!("{p}@example.org"))
                    .into_iter()
                    .collect(),
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
            },
            mailboxes: mailboxes.iter().map(|m| m.to_string()).collect(),
            matched: None,
            snippet: None,
        }
    }

    /// Each message hangs under the earlier one it replies to, and one
    /// whose parent the account lacks stands as a root of its own.
    #[test]
    fn replies_hang_under_their_parent() {
        let output = PimdirThreadOutput {
            style: PimdirListStyle {
                datetime_fmt: String::from("%F %R%:z"),
                datetime_local_tz: false,
            },
            thread: PimdirMailThread {
                thread: Some(String::from("t1")),
                count: 4,
                first: None,
                last: None,
                hits: vec![
                    hit("1", None, true, "Me", &["INBOX"]),
                    hit("2", Some("1"), false, "Alice", &["INBOX", "Archive"]),
                    hit("3", Some("2"), true, "Me", &["Sent"]),
                    hit("4", Some("0"), true, "Bob", &["INBOX"]),
                ],
            },
        };

        let expected = "\
Lunch
│  2026-11-11 10:00+01:00  1  in INBOX  Me <me@example.org>
└─ Re: Lunch
   │  2026-11-11 10:00+01:00  2 (unread)  in INBOX, Archive  Alice <alice@example.org>
   └─ Re: Lunch
      2026-11-11 10:00+01:00  3  in Sent  Me <me@example.org>

Re: Lunch
   2026-11-11 10:00+01:00  4  in INBOX  Bob <bob@example.org>
";
        assert_eq!(output.to_string(), expected);
    }
}
