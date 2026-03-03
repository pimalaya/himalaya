use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{presets, Cell, ContentArrangement, Row, Table};
use io_imap::coroutines::status::*;
use io_imap::types::status::{StatusDataItem, StatusDataItemName};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::{Serialize, Serializer};

use crate::{config::ImapConfig, imap::mailbox::arg::name::MailboxNameArg, imap::stream};

/// Get the status of a mailbox.
///
/// This command displays status information about a mailbox,
/// including message counts and UID values.
#[derive(Debug, Parser)]
pub struct StatusMailboxCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
}

impl StatusMailboxCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        let (context, mut stream) = stream::connect(config)?;

        let mailbox = self.mailbox.name.try_into()?;
        let item_names = vec![
            StatusDataItemName::Messages,
            StatusDataItemName::Recent,
            StatusDataItemName::Unseen,
            StatusDataItemName::UidNext,
            StatusDataItemName::UidValidity,
        ];

        let mut arg = None;
        let mut coroutine = ImapStatus::new(context, mailbox, item_names);

        let items = loop {
            match coroutine.resume(arg.take()) {
                ImapStatusResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapStatusResult::Ok { items, .. } => break items,
                ImapStatusResult::Err { err, .. } => bail!(err),
            }
        };

        let table = MailboxStatusTable::from(items);

        printer.out(table)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MailboxStatus {
    pub messages: Option<u32>,
    pub recent: Option<u32>,
    pub unseen: Option<u32>,
    pub uidnext: Option<u32>,
    pub uidvalidity: Option<u32>,
}

impl From<Vec<StatusDataItem>> for MailboxStatus {
    fn from(items: Vec<StatusDataItem>) -> Self {
        let mut status = MailboxStatus {
            messages: None,
            recent: None,
            unseen: None,
            uidnext: None,
            uidvalidity: None,
        };

        for item in items {
            match item {
                StatusDataItem::Messages(n) => status.messages = Some(n),
                StatusDataItem::Recent(n) => status.recent = Some(n),
                StatusDataItem::Unseen(n) => status.unseen = Some(n),
                StatusDataItem::UidNext(n) => status.uidnext = Some(n.get()),
                StatusDataItem::UidValidity(n) => status.uidvalidity = Some(n.get()),
                _ => {}
            }
        }

        status
    }
}

pub struct MailboxStatusTable {
    status: MailboxStatus,
}

impl From<Vec<StatusDataItem>> for MailboxStatusTable {
    fn from(items: Vec<StatusDataItem>) -> Self {
        Self {
            status: MailboxStatus::from(items),
        }
    }
}

impl fmt::Display for MailboxStatusTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([Cell::new("ATTRIBUTE"), Cell::new("VALUE")]));

        if let Some(messages) = self.status.messages {
            table.add_row(Row::from([Cell::new("Messages"), Cell::new(messages)]));
        }
        if let Some(recent) = self.status.recent {
            table.add_row(Row::from([Cell::new("Recent"), Cell::new(recent)]));
        }
        if let Some(unseen) = self.status.unseen {
            table.add_row(Row::from([Cell::new("Unseen"), Cell::new(unseen)]));
        }
        if let Some(uidnext) = self.status.uidnext {
            table.add_row(Row::from([Cell::new("UIDNext"), Cell::new(uidnext)]));
        }
        if let Some(uidvalidity) = self.status.uidvalidity {
            table.add_row(Row::from([
                Cell::new("UIDValidity"),
                Cell::new(uidvalidity),
            ]));
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for MailboxStatusTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.status.serialize(serializer)
    }
}
