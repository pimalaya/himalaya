//! # pimdir queue list
//!
//! The `pimdir queue list` command, tabling the creations staged in one
//! mailbox.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, Row, Table};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::envelope::Envelope,
    pimdir::client::PimdirClient,
    shared::{
        envelope::list::{format_addresses, format_flags},
        mailbox::arg::MailboxArg,
        table::style_from_preset,
    },
};

/// List the messages staged for creation in a mailbox.
///
/// A saved message waits in the queue until the sync engine applies it and
/// has no id until then, so `envelope list` cannot show it. This is where it
/// shows: the row id to cancel it by, when it was queued, and the mail.
///
/// Staged flags, moves and deletions need no such view, addressing messages
/// that already exist.
#[derive(Debug, Parser)]
pub struct PimdirQueueListCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
}

impl PimdirQueueListCommand {
    /// Lists the creations staged in the mailbox and tables them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account)?;
        let queued = client.queued_envelopes(&mailbox)?;

        printer.out(PimdirQueuedMessages {
            preset: account.table_preset().to_string(),
            id_color: account.envelopes_list_table_id_color(),
            subject_color: account.envelopes_list_table_subject_color(),
            from_color: account.envelopes_list_table_from_color(),
            date_color: account.envelopes_list_table_date_color(),
            unseen_char: account.envelopes_list_table_unseen_char(),
            replied_char: account.envelopes_list_table_replied_char(),
            flagged_char: account.envelopes_list_table_flagged_char(),
            messages: queued
                .into_iter()
                .map(|queued| PimdirQueuedMessage {
                    id: queued.id,
                    queued_at: queued.created_at,
                    producer: queued.producer,
                    envelope: queued.envelope,
                })
                .collect(),
        })
    }
}

/// One message waiting in the store's queue.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct PimdirQueuedMessage {
    /// The queue row id, which `pimdir queue cancel` takes. It names a
    /// pending action, not a message: the message has no id until the sync
    /// engine applies the action, and gets a different one then.
    pub id: i64,
    /// When the row was appended, stamped by the store's own clock.
    pub queued_at: String,
    /// The process that staged it.
    pub producer: String,
    /// The mail the action carries, read from its stored summary. Its `id` is
    /// empty, a queued message having none yet.
    pub envelope: Envelope,
}

/// The `pimdir queue list` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct PimdirQueuedMessages {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// Color of the ID column.
    #[serde(skip)]
    pub id_color: Color,
    /// Color of the SUBJECT column.
    #[serde(skip)]
    pub subject_color: Color,
    /// Color of the FROM column.
    #[serde(skip)]
    pub from_color: Color,
    /// Color of the DATE column.
    #[serde(skip)]
    pub date_color: Color,
    /// FLAGS glyph of a message lacking `\Seen`.
    #[serde(skip)]
    pub unseen_char: char,
    /// FLAGS glyph of a message carrying `\Answered`.
    #[serde(skip)]
    pub replied_char: char,
    /// FLAGS glyph of a message carrying `\Flagged`.
    #[serde(skip)]
    pub flagged_char: char,
    /// The messages staged for creation in the mailbox.
    pub messages: Vec<PimdirQueuedMessage>,
}

impl fmt::Display for PimdirQueuedMessages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.messages.is_empty() {
            return writeln!(f, "No message queued in this mailbox");
        }

        let chars = crate::shared::envelope::list::FlagChars {
            unseen: self.unseen_char,
            replied: self.replied_char,
            flagged: self.flagged_char,
            attachment: ' ',
        };

        let mut table = Table::new();
        table
            .load_style(style_from_preset(&self.preset))
            .set_header(Row::from([
                Cell::new("ROW"),
                Cell::new("FLAGS"),
                Cell::new("SUBJECT"),
                Cell::new("TO"),
                Cell::new("QUEUED"),
            ]))
            .add_rows(self.messages.iter().map(|queued| {
                let mut row = Row::new();
                row.max_height(1);
                row.add_cell(Cell::new(queued.id).fg(self.id_color));
                row.add_cell(Cell::new(format_flags(&queued.envelope.flags, &chars)));
                row.add_cell(Cell::new(&queued.envelope.subject).fg(self.subject_color));
                row.add_cell(Cell::new(format_addresses(&queued.envelope.to)).fg(self.from_color));
                row.add_cell(Cell::new(&queued.queued_at).fg(self.date_color));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")?;
        writeln!(
            f,
            "Queued until the next sync. Cancel one with `himalaya pimdir queue cancel <ROW>`"
        )
    }
}
