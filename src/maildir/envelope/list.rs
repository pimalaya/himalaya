use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_fs::runtimes::std::handle;
use io_maildir::{coroutines::list_messages::*, maildir::Maildir};
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::maildir::{account::MaildirAccount, arg::MaildirPathFlag};

/// List MAILDIR envelopes from the given mailbox.
///
/// This command displays envelopes for messages in the specified
/// mailbox. You can specify a sequence set to limit which messages
/// are fetched.
#[derive(Debug, Parser)]
pub struct MaildirEnvelopeListCommand {
    #[command(flatten)]
    pub maildir: MaildirPathFlag,
}

impl MaildirEnvelopeListCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.maildir.inner))?,
        };

        let mut arg = None;
        let mut coroutine = ListMaildirMessages::new(maildir);

        let messages = loop {
            match coroutine.resume(arg.take()) {
                ListMaildirMessagesResult::Io(io) => arg = Some(handle(io)?),
                ListMaildirMessagesResult::Ok(messages) => break messages,
                ListMaildirMessagesResult::Err(err) => bail!(err),
            };
        };

        let mut envelopes = Vec::with_capacity(messages.len());

        for message in messages {
            let Some(id) = message.id() else {
                continue;
            };

            let Some(headers) = message.headers() else {
                continue;
            };

            let mut row = EnvelopesTableEntry::default();

            row.id = id.to_owned();
            row.subject = headers.subject().unwrap_or("").to_owned();

            if let Some(addr) = headers.from().and_then(|a| a.first()) {
                row.from = addr.name().or(addr.address()).unwrap_or("").to_owned();
            }

            if let Some(date) = headers.date() {
                row.date = date.to_rfc822();
            }

            envelopes.push(row);
        }

        envelopes.sort_by(|a, b| a.date.cmp(&b.date));

        let table = EnvelopesTable {
            preset: account.table_preset,
            arrangement: account.table_arrangement,
            envelopes,
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EnvelopesTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    envelopes: Vec<EnvelopesTableEntry>,
}

impl fmt::Display for EnvelopesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("SUBJECT"),
                Cell::new("FROM"),
                Cell::new("DATE"),
            ]));

        for entry in &self.envelopes {
            let mut row = Row::new();

            row.max_height(1)
                .add_cell(Cell::new(&entry.id))
                .add_cell(Cell::new(&entry.subject))
                .add_cell(Cell::new(&entry.from))
                .add_cell(Cell::new(&entry.date));

            table.add_row(row);
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct EnvelopesTableEntry {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub date: String,
}
