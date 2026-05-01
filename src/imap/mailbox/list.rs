use std::{
    fmt,
    io::{Read, Write},
};

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_imap::{
    rfc3501::{list::*, lsub::*},
    types::{core::QuotedChar, flag::FlagNameAttribute, mailbox::Mailbox},
};
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::imap::account::ImapAccount;

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// List, search and filter mailboxes.
///
/// This command allows you to list mailboxes from your IMAP account.
/// By default, only subscribed mailboxes are listed. Use --all to
/// list all mailboxes.
#[derive(Debug, Parser)]
pub struct ImapMailboxListCommand {
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

impl ImapMailboxListCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let reference = self.reference.try_into()?;
        let pattern = self.pattern.try_into()?;

        let mut buf = [0u8; READ_BUFFER_SIZE];

        let mailboxes = if self.all {
            let mut coroutine = ImapMailboxList::new(imap.context, reference, pattern);
            let mut arg: Option<&[u8]> = None;

            loop {
                match coroutine.resume(arg.take()) {
                    ImapMailboxListResult::Ok { mailboxes, .. } => break mailboxes,
                    ImapMailboxListResult::WantsRead => {
                        let n = imap.stream.read(&mut buf)?;
                        arg = Some(&buf[..n]);
                    }
                    ImapMailboxListResult::WantsWrite(bytes) => {
                        imap.stream.write_all(&bytes)?;
                        arg = None;
                    }
                    ImapMailboxListResult::Err { err, .. } => bail!("{err}"),
                }
            }
        } else {
            let mut coroutine = ImapMailboxLsub::new(imap.context, reference, pattern);
            let mut arg: Option<&[u8]> = None;

            loop {
                match coroutine.resume(arg.take()) {
                    ImapMailboxLsubResult::Ok { mailboxes, .. } => break mailboxes,
                    ImapMailboxLsubResult::WantsRead => {
                        let n = imap.stream.read(&mut buf)?;
                        arg = Some(&buf[..n]);
                    }
                    ImapMailboxLsubResult::WantsWrite(bytes) => {
                        imap.stream.write_all(&bytes)?;
                        arg = None;
                    }
                    ImapMailboxLsubResult::Err { err, .. } => bail!("{err}"),
                }
            }
        };

        let table = MailboxesTable {
            preset: account.table_preset,
            mailboxes: mailboxes.into_iter().map(From::from).collect(),
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct MailboxesTable {
    #[serde(skip)]
    pub preset: String,
    pub mailboxes: Vec<MailboxRow>,
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
            .add_rows(self.mailboxes.iter().map(|mbox| {
                let mut row = Row::new();

                row.max_height(1)
                    .add_cell(Cell::new(&mbox.name))
                    .add_cell(Cell::new(&mbox.delimiter))
                    .add_cell(Cell::new(mbox.attributes.join(", ")));

                row
            }));

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
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
