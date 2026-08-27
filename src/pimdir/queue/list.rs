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
/// A saved message waits in the store's queue until the sync engine applies it,
/// and has no id until then, so it cannot appear in `envelope list`. This is
/// where it shows: the row id to cancel it by, when it was queued, and the mail
/// itself. Staged flags, moves and deletions need no such view, since they
/// address messages that already exist and show in the ordinary listing.
#[derive(Debug, Parser)]
pub struct PimdirQueueListCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
}

impl PimdirQueueListCommand {
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
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub id_color: Color,
    #[serde(skip)]
    pub subject_color: Color,
    #[serde(skip)]
    pub from_color: Color,
    #[serde(skip)]
    pub date_color: Color,
    #[serde(skip)]
    pub unseen_char: char,
    #[serde(skip)]
    pub replied_char: char,
    #[serde(skip)]
    pub flagged_char: char,
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
