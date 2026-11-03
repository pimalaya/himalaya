//! # IMAP select
//!
//! The `imap select` command, RFC 3501 `SELECT`.

use std::{fmt, num::NonZeroU32};

use anyhow::Result;
use clap::Parser;
use io_imap::{
    client::ImapClient as _, rfc3501::select::ImapMailboxSelectOptions, types::flag::FlagPerm,
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg, outcome};

/// Select the given mailbox (SELECT, RFC 3501).
///
/// Opens it for read-write access and returns its status. The selection
/// only outlives the command over a stateful IMAP session such as
/// `himalaya repl` or a Sirup proxy.
#[derive(Debug, Parser)]
pub struct ImapMailboxSelectCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxSelectCommand {
    /// Selects the mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let name = self.mailbox_name.inner;
        let mailbox = name.clone().try_into()?;

        let result = client
            .select(mailbox, ImapMailboxSelectOptions::default())
            .map_err(Into::into)
            .map(|data| ImapMailboxSelectOutput {
                mailbox: name,
                flags: data
                    .flags
                    .unwrap_or_default()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                exists: data.exists,
                recent: data.recent,
                unseen: data.unseen.map(NonZeroU32::get),
                permanent_flags: data
                    .permanent_flags
                    .unwrap_or_default()
                    .iter()
                    .map(|flag| match flag {
                        FlagPerm::Flag(flag) => flag.to_string(),
                        FlagPerm::Asterisk => "\\*".to_string(),
                    })
                    .collect(),
                uid_next: data.uid_next.map(NonZeroU32::get),
                uid_validity: data.uid_validity.map(NonZeroU32::get),
                highest_mod_seq: data.highest_mod_seq,
            });

        outcome::print(printer, result)
    }
}

/// What `imap select` understood of the server's answer.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImapMailboxSelectOutput {
    /// The mailbox selected.
    mailbox: String,
    /// The flags defined in the mailbox, as spelled on the wire.
    flags: Vec<String>,
    /// How many messages the mailbox holds, `null` when not sent.
    exists: Option<u32>,
    /// How many messages are recent, `null` when not sent.
    recent: Option<u32>,
    /// The number of the first unseen message, `null` when not sent.
    unseen: Option<u32>,
    /// The flags a client can change permanently; `\\*` allows new
    /// keywords.
    permanent_flags: Vec<String>,
    /// The next UID, `null` when not sent.
    uid_next: Option<u32>,
    /// The UID validity, `null` when not sent.
    uid_validity: Option<u32>,
    /// The highest mod-sequence (CONDSTORE), `null` when not sent.
    highest_mod_seq: Option<u64>,
}

impl fmt::Display for ImapMailboxSelectOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Mailbox successfully selected: {}", self.mailbox)?;

        if let Some(exists) = self.exists {
            writeln!(f, "  Messages: {exists}")?;
        }
        if let Some(uid_validity) = self.uid_validity {
            writeln!(f, "  UID validity: {uid_validity}")?;
        }
        if let Some(uid_next) = self.uid_next {
            writeln!(f, "  UID next: {uid_next}")?;
        }

        Ok(())
    }
}
