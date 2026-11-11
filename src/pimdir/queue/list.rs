//! # pimdir queue list
//!
//! The `pimdir queue list` command, listing the creations and sends staged
//! in one mailbox.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    pimdir::{backend::PimdirQueuedEnvelope, client::PimdirClient},
    shared::{
        envelope::list::{format_addresses, format_flags},
        mailbox::arg::MailboxArg,
    },
};

/// List the messages staged for creation or sending in a mailbox.
///
/// A saved message waits in the queue until the sync engine applies it and
/// has no id until then, so `envelope list` cannot show it. A sent message
/// waits there until the store's owner sends it, filed under the mailbox it
/// was saved to or the account's sent alias. This is where both show: the
/// row id to cancel one by, when it was queued, and the mail.
///
/// Staged flags, moves and deletions need no such view, addressing messages
/// that already exist. A row of any kind the sync engine parked, refusing to
/// apply it without an operator, is listed under them with why: cancel it,
/// or fix what parked it.
#[derive(Debug, Parser)]
pub struct PimdirQueueListCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
}

impl PimdirQueueListCommand {
    /// Lists the creations and sends staged in the mailbox and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account);
        let queued = client.queued_envelopes(&mailbox)?;
        let parked = client.parked_actions(&mailbox)?;

        printer.out(PimdirQueueListOutput {
            messages: queued
                .into_iter()
                .map(|queued| PimdirQueuedMessage {
                    id: queued.id,
                    queued_at: queued.created_at,
                    producer: queued.producer,
                    send: queued.send,
                    envelope: queued.envelope,
                })
                .collect(),
            parked: parked
                .into_iter()
                .map(|parked| PimdirParkedRow {
                    id: parked.id,
                    queued_at: parked.created_at,
                    producer: parked.producer,
                    kind: parked.action,
                    attempts: parked.attempts,
                    error: parked.error,
                })
                .collect(),
        })
    }
}

/// One message waiting in the store's queue.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueuedMessage {
    /// The queue row id, which `pimdir queue cancel` takes. It names a
    /// pending action, not a message: the message has no id until the sync
    /// engine applies the action, and gets a different one then.
    #[serde(rename = "row")]
    pub id: i64,
    /// When the row was appended, stamped by the store's own clock.
    pub queued_at: String,
    /// The process that staged it.
    pub producer: String,
    /// Whether the row sends the message rather than files it.
    pub send: bool,
    /// The mail the action carries, derived from the body it pins. Its `id`
    /// is `null`, a queued message having none yet.
    pub envelope: PimdirQueuedEnvelope,
}

/// One row the sync engine parked: it will not apply it again without an
/// operator.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirParkedRow {
    /// The queue row id, which `pimdir queue cancel` takes.
    #[serde(rename = "row")]
    pub id: i64,
    /// When the row was appended, stamped by the store's own clock.
    pub queued_at: String,
    /// The process that staged it.
    pub producer: String,
    /// The action kind: `add`, `submit`, `set-flags`, ...
    pub kind: String,
    /// How many times the engine tried it before parking it.
    pub attempts: i64,
    /// Why it was parked.
    pub error: String,
}

/// The `pimdir queue list` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueueListOutput {
    /// The messages staged for creation in the mailbox.
    pub messages: Vec<PimdirQueuedMessage>,
    /// The mailbox's parked rows, any kind.
    pub parked: Vec<PimdirParkedRow>,
}

impl fmt::Display for PimdirQueueListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.messages.is_empty() && self.parked.is_empty() {
            return writeln!(f, "No message queued in this mailbox");
        }
        if !self.messages.is_empty() {
            self.fmt_queued(f)?;
        }
        if !self.messages.is_empty() && !self.parked.is_empty() {
            writeln!(f)?;
        }
        if !self.parked.is_empty() {
            self.fmt_parked(f)?;
        }
        Ok(())
    }
}

impl PimdirQueueListOutput {
    /// The list of queued creations and sends.
    fn fmt_queued(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: the subject is the one free-text column, so it reads last.
        let header = ["ROW", "ACTION", "FLAGS", "QUEUED", "TO", "SUBJECT"];
        let mut columns = Columns::new(header);

        for queued in &self.messages {
            let to = format_addresses(&queued.envelope.to);

            columns.push([
                queued.id.to_string(),
                String::from(if queued.send { "send" } else { "save" }),
                format_flags(&queued.envelope.flags),
                sanitize::text(&queued.queued_at).into_owned(),
                sanitize::text(&to).into_owned(),
                sanitize::text(&queued.envelope.subject).into_owned(),
            ]);
        }

        write!(f, "{columns}")?;
        writeln!(
            f,
            "Queued until the next sync. Cancel one with `himalaya pimdir queue cancel <ROW>`"
        )
    }

    /// The list of parked rows.
    fn fmt_parked(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::new(["ROW", "ACTION", "QUEUED", "ERROR"]);

        for parked in &self.parked {
            columns.push([
                parked.id.to_string(),
                sanitize::text(&parked.kind).into_owned(),
                sanitize::text(&parked.queued_at).into_owned(),
                sanitize::text(&parked.error).into_owned(),
            ]);
        }

        write!(f, "{columns}")?;
        writeln!(
            f,
            "Parked by the sync engine, which will not retry them. Cancel one with \
             `himalaya pimdir queue cancel <ROW>`"
        )
    }
}
