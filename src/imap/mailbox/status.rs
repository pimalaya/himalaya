//! # IMAP status
//!
//! The `imap status` command, RFC 3501 `STATUS`.

use io_imap::client::ImapClient as _;
use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_imap::types::status::{StatusDataItem, StatusDataItemName};
use pimalaya_cli::{fields::Fields, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    imap::{client::ImapClient, mailbox::arg::MailboxNameArg, outcome, utils::output_data},
};

/// Get the status of the given mailbox (STATUS, RFC 3501).
///
/// Message counts and UID values, without selecting the mailbox.
#[derive(Debug, Parser)]
pub struct ImapMailboxStatusCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxStatusCommand {
    /// Asks for the RFC 3501 status items and prints what came back.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;
        let item_names = vec![
            StatusDataItemName::Messages,
            StatusDataItemName::Recent,
            StatusDataItemName::Unseen,
            StatusDataItemName::UidNext,
            StatusDataItemName::UidValidity,
        ];

        let result = (|| -> Result<_> {
            let items = client.status(mailbox, item_names.into())?;

            Ok(ImapMailboxStatusOutput::from(output_data("STATUS", items)))
        })();

        outcome::print(printer, result)
    }
}

/// The `imap status` output, one field per data item the server returned.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImapMailboxStatusOutput {
    /// Total message count.
    pub messages: Option<u32>,
    /// Count of messages carrying `\Recent`.
    pub recent: Option<u32>,
    /// The UID the next message will be assigned.
    pub uid_next: Option<u32>,
    /// The UID validity, which changes when every UID does.
    pub uid_validity: Option<u32>,
    /// Count of messages lacking `\Seen`.
    pub unseen: Option<u32>,
}

impl From<Vec<StatusDataItem>> for ImapMailboxStatusOutput {
    fn from(items: Vec<StatusDataItem>) -> Self {
        let mut status = ImapMailboxStatusOutput {
            messages: None,
            recent: None,
            uid_next: None,
            uid_validity: None,
            unseen: None,
        };

        for item in items {
            match item {
                StatusDataItem::Messages(n) => status.messages = Some(n),
                StatusDataItem::Recent(n) => status.recent = Some(n),
                StatusDataItem::UidNext(n) => status.uid_next = Some(n.get()),
                StatusDataItem::UidValidity(n) => status.uid_validity = Some(n.get()),
                StatusDataItem::Unseen(n) => status.unseen = Some(n),
                StatusDataItem::Deleted(_)
                | StatusDataItem::DeletedStorage(_)
                | StatusDataItem::HighestModSeq(_) => {}
            }
        }

        status
    }
}

impl fmt::Display for ImapMailboxStatusOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rows = [
            ("Messages", self.messages.map(u64::from)),
            ("Recent", self.recent.map(u64::from)),
            ("Unseen", self.unseen.map(u64::from)),
            ("UID next", self.uid_next.map(u64::from)),
            ("UID validity", self.uid_validity.map(u64::from)),
        ];

        let mut fields = Fields::default();

        for (label, value) in rows {
            if let Some(value) = value {
                fields.push(label, value);
            }
        }

        write!(f, "{fields}")
    }
}

#[cfg(test)]
mod tests {
    use io_imap::types::status::StatusDataItem;

    use crate::imap::mailbox::status::ImapMailboxStatusOutput;

    /// The output prints the items the command asks for, and no key it
    /// never requests, which would always be `null`.
    #[test]
    fn the_status_prints_the_items_it_asks_for() {
        let output = ImapMailboxStatusOutput::from(vec![
            StatusDataItem::Messages(3),
            StatusDataItem::Unseen(1),
        ]);

        let json = serde_json::to_value(&output).unwrap();
        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            ["messages", "recent", "uidNext", "uidValidity", "unseen"]
        );
    }
}
