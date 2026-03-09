use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_imap::{
    coroutines::{list::*, lsub::*},
    types::{core::QuotedChar, flag::FlagNameAttribute, mailbox::Mailbox},
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::{Serialize, Serializer};

use crate::imap::{account::ImapAccount, stream};

/// List, search and filter mailboxes.
///
/// This command allows you to list mailboxes from your IMAP account.
/// By default, only subscribed mailboxes are listed. Use --all to
/// list all mailboxes.
#[derive(Debug, Parser)]
pub struct ListMailboxesCommand {
    /// List all mailboxes, not just subscribed ones.
    #[arg(short = 'A', long)]
    pub all: bool,

    /// The reference name for the LIST/LSUB command.
    #[arg(short, long, default_value = "")]
    pub reference: String,

    /// The mailbox name pattern with wildcards (* and %).
    #[arg(short, long, default_value = "*")]
    pub pattern: String,
}

impl ListMailboxesCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let reference = self.reference.try_into()?;
        let pattern = self.pattern.try_into()?;

        let mailboxes = if self.all {
            let mut arg = None;
            let mut coroutine = ImapList::new(context, reference, pattern);

            loop {
                match coroutine.resume(arg.take()) {
                    ImapListResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                    ImapListResult::Ok { mailboxes, .. } => break mailboxes,
                    ImapListResult::Err { err, .. } => bail!(err),
                }
            }
        } else {
            let mut arg = None;
            let mut coroutine = ImapLsub::new(context, reference, pattern);

            loop {
                match coroutine.resume(arg.take()) {
                    ImapLsubResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                    ImapLsubResult::Ok { mailboxes, .. } => break mailboxes,
                    ImapLsubResult::Err { err, .. } => bail!(err),
                }
            }
        };

        let table = MailboxesTable {
            preset: account.table_preset,
            rows: mailboxes.into_iter().map(From::from).collect(),
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Default)]
pub struct MailboxesTable {
    pub preset: String,
    pub rows: Vec<MailboxRow>,
}

impl fmt::Display for MailboxesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([
                Cell::new("NAME"),
                Cell::new("DELIMITER"),
                Cell::new("ATTRIBUTES"),
            ]))
            .add_rows(self.rows.iter().map(|mbox| {
                let mut row = Row::new();

                row.max_height(1)
                    .add_cell(Cell::new(&mbox.name))
                    .add_cell(Cell::new(&mbox.delimiter))
                    .add_cell(Cell::new(&mbox.attributes.join(", ")));

                row
            }));

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for MailboxesTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.rows.serialize(serializer)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MailboxRow {
    pub name: String,
    pub delimiter: String,
    pub attributes: Vec<String>,
}

impl
    From<(
        Mailbox<'static>,
        Option<QuotedChar>,
        Vec<FlagNameAttribute<'static>>,
    )> for MailboxRow
{
    fn from(
        (mbox, delim, attrs): (
            Mailbox<'static>,
            Option<QuotedChar>,
            Vec<FlagNameAttribute<'static>>,
        ),
    ) -> Self {
        Self {
            name: match mbox {
                Mailbox::Inbox => "Inbox".into(),
                Mailbox::Other(mbox) => String::from_utf8_lossy(mbox.inner().as_ref()).to_string(),
            },
            delimiter: match delim {
                Some(delim) => delim.inner().to_string(),
                None => String::new(),
            },
            attributes: attrs.into_iter().map(|attr| attr.to_string()).collect(),
        }
    }
}
