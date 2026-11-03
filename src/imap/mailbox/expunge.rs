//! # IMAP expunge
//!
//! The `imap expunge` command, RFC 3501 `EXPUNGE`.

use std::{fmt, num::NonZeroU32};

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use io_imap::rfc3501::select::ImapMailboxSelectOptions;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameArg, MailboxNoSelectFlag},
    outcome,
};

/// Expunge the given mailbox (EXPUNGE, RFC 3501).
///
/// Every message flagged `\Deleted` goes for good.
#[derive(Debug, Parser)]
pub struct ImapMailboxExpungeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,
}

impl ImapMailboxExpungeCommand {
    /// Selects the mailbox unless told not to, then expunges it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        let result = (|| -> Result<_> {
            if !self.mailbox_no_select.inner {
                client.select(mailbox, ImapMailboxSelectOptions::default())?;
            }

            let expunged = client.expunge()?;

            Ok(ImapMailboxExpungeOutput {
                expunged: expunged.into_iter().map(NonZeroU32::get).collect(),
            })
        })();

        outcome::print(printer, result)
    }
}

/// What `imap expunge` understood of the server's answer.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImapMailboxExpungeOutput {
    /// The message numbers expunged, in the order the server sent them.
    expunged: Vec<u32>,
}

impl fmt::Display for ImapMailboxExpungeOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.expunged.len();
        writeln!(
            f,
            "Mailbox successfully expunged: {count} message(s) removed"
        )
    }
}
