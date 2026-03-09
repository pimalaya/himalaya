use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{presets, Cell, ContentArrangement, Row, Table};
use io_imap::{
    coroutines::status::*,
    types::status::{StatusDataItem, StatusDataItemName},
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::{Serialize, Serializer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg, stream};

/// Get the status of the given mailbox.
///
/// This command displays status information about a mailbox,
/// including message counts and UID values.
#[derive(Debug, Parser)]
pub struct StatusMailboxCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
}

impl StatusMailboxCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

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
    pub uid_next: Option<u32>,
    pub uid_validity: Option<u32>,
    pub unseen: Option<u32>,
    pub deleted: Option<u32>,
    pub deleted_storage: Option<u64>,
    pub highest_mod_seq: Option<u64>,
}

impl From<Vec<StatusDataItem>> for MailboxStatus {
    fn from(items: Vec<StatusDataItem>) -> Self {
        let mut status = MailboxStatus {
            messages: None,
            recent: None,
            uid_next: None,
            uid_validity: None,
            unseen: None,
            deleted: None,
            deleted_storage: None,
            highest_mod_seq: None,
        };

        for item in items {
            match item {
                StatusDataItem::Messages(n) => status.messages = Some(n),
                StatusDataItem::Recent(n) => status.recent = Some(n),
                StatusDataItem::UidNext(n) => status.uid_next = Some(n.get()),
                StatusDataItem::UidValidity(n) => status.uid_validity = Some(n.get()),
                StatusDataItem::Unseen(n) => status.unseen = Some(n),
                StatusDataItem::Deleted(n) => status.deleted = Some(n),
                StatusDataItem::DeletedStorage(n) => status.deleted_storage = Some(n),
                StatusDataItem::HighestModSeq(n) => status.highest_mod_seq = Some(n),
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

        if let Some(n) = self.status.messages {
            table.add_row(Row::from([Cell::new("Messages"), Cell::new(n)]));
        }

        if let Some(n) = self.status.recent {
            table.add_row(Row::from([Cell::new("Recent"), Cell::new(n)]));
        }

        if let Some(n) = self.status.unseen {
            table.add_row(Row::from([Cell::new("Unseen"), Cell::new(n)]));
        }

        if let Some(n) = self.status.deleted {
            table.add_row(Row::from([Cell::new("Deleted"), Cell::new(n)]));
        }

        if let Some(n) = self.status.deleted_storage {
            table.add_row(Row::from([Cell::new("Deleted storage"), Cell::new(n)]));
        }

        if let Some(deleted) = self.status.highest_mod_seq {
            table.add_row(Row::from([
                Cell::new("Highest modified sequence"),
                Cell::new(deleted),
            ]));
        }

        if let Some(n) = self.status.uid_next {
            table.add_row(Row::from([Cell::new("UID next"), Cell::new(n)]));
        }

        if let Some(n) = self.status.uid_validity {
            table.add_row(Row::from([Cell::new("UID validity"), Cell::new(n)]));
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

impl Serialize for MailboxStatusTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.status.serialize(serializer)
    }
}
