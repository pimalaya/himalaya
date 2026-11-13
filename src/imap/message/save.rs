//! # IMAP append
//!
//! The `imap append` command, RFC 3501 `APPEND`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use io_imap::{
    rfc3501::append::ImapMessageAppendOptions,
    types::{IntoStatic, flag::Flag, mailbox::Mailbox},
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    imap::{client::ImapClient, mailbox::arg::MailboxNameArg, outcome},
    shared::message::arg::MessageArg,
};

/// Append a message to a mailbox (APPEND, RFC 3501).
///
/// The message comes from a file path, an inline string or piped standard
/// input.
#[derive(Debug, Parser)]
pub struct ImapMessageSaveCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
    /// Flags to set on the appended message, as raw RFC 3501 tokens.
    ///
    /// This is the raw IMAP API, not the unified
    /// `seen|answered|flagged|draft` enum: a system flag keeps its
    /// backslash, as in `-f '\Seen'`, and a bare word is a custom
    /// keyword, so `-f seen` stores the keyword `seen` and `imap search
    /// --seen` will not match it.
    ///
    /// The unified `message add -f seen` is the enum-mapped behaviour.
    #[arg(short, long)]
    pub flag: Vec<String>,
    #[command(flatten)]
    pub message: MessageArg,
}

impl ImapMessageSaveCommand {
    /// Appends the message to the mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox: Mailbox<'static> = self.mailbox.inner.try_into()?;
        let message = self.message.parse()?;

        let flags: Vec<Flag<'static>> = self
            .flag
            .iter()
            .map(String::as_str)
            .map(|f| Flag::try_from(f).map(IntoStatic::into_static))
            .collect::<Result<_, _>>()?;

        let result = client
            .append(
                mailbox,
                &message,
                ImapMessageAppendOptions {
                    flags,
                    date: None,
                    non_sync: false,
                },
            )
            .map_err(Into::into)
            .map(|(exists, append_uid)| ImapMessageAppendOutput {
                uid_validity: append_uid.map(|(validity, _)| validity),
                uid: append_uid.map(|(_, uid)| uid),
                exists,
            });

        outcome::print(printer, result)
    }
}

/// What `imap append` understood of the server's answer.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImapMessageAppendOutput {
    /// The UID validity of the mailbox, from `APPENDUID` (UIDPLUS),
    /// `null` when the server sent none.
    uid_validity: Option<u32>,
    /// The UID of the appended message, from `APPENDUID` (UIDPLUS),
    /// `null` when the server sent none.
    uid: Option<u32>,
    /// How many messages the mailbox holds, `null` when not sent.
    exists: Option<u32>,
}

impl fmt::Display for ImapMessageAppendOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.uid {
            Some(uid) => writeln!(f, "Message successfully appended: UID {uid}"),
            None => writeln!(f, "Message successfully appended"),
        }
    }
}
