//! # Mailbox list
//!
//! The `mailbox list` command, listing the mailboxes of the active
//! account.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, email::mailbox::Mailbox, shared::client::EmailClient};

/// List the mailboxes of the active account.
#[derive(Debug, Parser)]
pub struct MailboxListCommand {
    /// Fill the TOTAL and UNREAD columns.
    ///
    /// JMAP returns the counts in the same response, but IMAP issues one
    /// extra `STATUS` per mailbox, which is slow on an account with many.
    /// Maildir does not implement counts at all.
    #[arg(long)]
    pub counts: bool,
}

impl MailboxListCommand {
    /// Lists the mailboxes and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mut mailboxes = client.list_mailboxes(self.counts)?;
        account.apply_role_aliases(&mut mailboxes);

        let mailboxes = MailboxListOutput {
            with_counts: self.counts,
            mailboxes,
        };

        printer.out(mailboxes)
    }
}

/// The `mailbox list` output, the mailboxes of the account.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MailboxListOutput {
    /// Whether the count columns are drawn.
    #[serde(skip)]
    pub with_counts: bool,
    /// The mailboxes, in the order the backend returned them.
    pub mailboxes: Vec<Mailbox>,
}

impl fmt::Display for MailboxListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: a name is free text, so it reads last.
        let mut header = vec!["ID", "ROLE"];
        if self.with_counts {
            header.extend(["TOTAL", "UNREAD"]);
        }
        header.push("NAME");

        let mut columns = Columns::new(header);

        for mailbox in &self.mailboxes {
            let mut row = vec![
                sanitize::text(&mailbox.id).into_owned(),
                mailbox
                    .role
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            ];
            if self.with_counts {
                row.push(count(mailbox.total));
                row.push(count(mailbox.unread));
            }
            row.push(sanitize::text(&mailbox.name).into_owned());
            columns.push(row);
        }

        write!(f, "{columns}")
    }
}

/// Renders a count, or an empty cell when the backend gave none.
fn count(value: Option<u64>) -> String {
    value.map(|n| n.to_string()).unwrap_or_default()
}
