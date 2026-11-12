//! # IMAP list
//!
//! The `imap list` command, RFC 3501 `LIST` and `LSUB`.

use io_imap::client::ImapClient as _;
use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_imap::types::{core::QuotedChar, flag::FlagNameAttribute, mailbox::Mailbox};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    email::mailbox::MailboxRole,
    imap::{client::ImapClient, outcome},
    listing::Listing,
};

/// List mailboxes (LIST and LSUB, RFC 3501).
///
/// `LSUB` lists the subscribed mailboxes, which is the default, and
/// `--all` switches to the `LIST` of every one.
#[derive(Debug, Parser)]
pub struct ImapMailboxListCommand {
    /// List every mailbox rather than the subscribed ones.
    #[arg(short = 'A', long)]
    pub all: bool,
    /// The reference name the listing starts from.
    #[arg(short, long, default_value = "")]
    pub reference: String,
    /// The name pattern to match, `*` and `%` being the wildcards.
    #[arg(short, long, default_value = "*")]
    pub pattern: String,
}

impl ImapMailboxListCommand {
    /// Lists the mailboxes and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        let reference = self.reference.try_into()?;
        let pattern = self.pattern.try_into()?;

        let result = (|| -> Result<_> {
            let mailboxes = if self.all {
                client.list(reference, pattern)?
            } else {
                client.lsub(reference, pattern)?
            };

            Ok(ImapMailboxListOutput {
                account: account.name().to_string(),
                mailboxes: mailboxes.into_iter().map(From::from).collect(),
            })
        })();

        outcome::print(printer, result)
    }
}

/// The `imap list` output, the mailboxes the server lists.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImapMailboxListOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The mailboxes, in the order the server returned them.
    pub mailboxes: Vec<MailboxRow>,
}

impl fmt::Display for ImapMailboxListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: the attributes join without a blank, so a line still splits
        // on blanks.
        let mut listing = Listing::new(&self.account, Some(Backend::Imap));

        for mbox in &self.mailboxes {
            let role = mbox
                .attributes
                .iter()
                .find_map(|raw| match MailboxRole::parse(raw) {
                    MailboxRole::Other(_) => None,
                    role => Some(format!("{role:?}")),
                })
                .unwrap_or_default();

            listing.items.push([
                sanitize::text(&mbox.delimiter).into_owned(),
                role,
                sanitize::text(&mbox.attributes.join(",")).into_owned(),
                sanitize::text(&mbox.name).into_owned(),
            ]);
        }

        write!(f, "{listing}")
    }
}

/// One mailbox of the list.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MailboxRow {
    /// The mailbox name.
    pub name: String,
    /// The hierarchy delimiter, empty when the server reported none.
    pub delimiter: String,
    /// The name attributes the server reported, SPECIAL-USE included.
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
