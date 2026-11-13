//! # Mailbox list
//!
//! The `mailbox list` command, listing the mailboxes of the active
//! account.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    email::mailbox::Mailbox,
    listing::{self, Listing},
    shared::client::EmailClient,
};

/// List the mailboxes of the active account.
#[derive(Debug, Parser)]
pub struct MailboxListCommand {
    /// Print the unread and total counts before the name, as `3/120`.
    ///
    /// JMAP and Graph return the counts in the same response, but IMAP
    /// issues one extra `STATUS` per mailbox and Gmail one `labels.get` per
    /// label, which is slow on an account with many. Maildir does not
    /// implement counts at all.
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

        let resolution = client.resolution();

        printer.out(MailboxListOutput {
            account: resolution.account.clone(),
            backend: resolution.storage,
            with_counts: self.counts,
            mailboxes,
        })
    }
}

/// The `mailbox list` output, the mailboxes of the account.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MailboxListOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The storage backend the account resolved to.
    #[serde(skip)]
    pub backend: Option<Backend>,
    /// Whether the counts are printed.
    #[serde(skip)]
    pub with_counts: bool,
    /// The mailboxes, in the order the backend returned them.
    pub mailboxes: Vec<Mailbox>,
}

impl fmt::Display for MailboxListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, self.backend);

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
                row.push(listing::counts(mailbox.unread, mailbox.total));
            }
            row.push(sanitize::text(&mailbox.name).into_owned());
            listing.items.push(row);
        }

        write!(f, "{listing}")
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        email::mailbox::{Mailbox, MailboxRole},
        shared::mailbox::list::MailboxListOutput,
    };

    /// The counts print as one `unread/total` cell, and only with
    /// `--counts`.
    #[test]
    fn counts_print_as_unread_over_total() {
        let mailbox = |id: &str, role, counts: Option<(u64, u64)>| Mailbox {
            id: id.into(),
            name: id.into(),
            role,
            unread: counts.map(|(unread, _)| unread),
            total: counts.map(|(_, total)| total),
        };
        let output = |with_counts| MailboxListOutput {
            account: "work".into(),
            backend: None,
            with_counts,
            mailboxes: vec![
                mailbox("INBOX", Some(MailboxRole::Inbox), Some((3, 120))),
                mailbox("Lists", None, None),
            ],
        };

        let expected = "\
Account:    work

INBOX  inbox  3/120  INBOX
Lists  -      -      Lists
";
        assert_eq!(output(true).to_string(), expected);

        let expected = "\
Account:    work

INBOX  inbox  INBOX
Lists  -      Lists
";
        assert_eq!(output(false).to_string(), expected);
    }
}
