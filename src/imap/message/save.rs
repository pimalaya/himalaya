//! # IMAP append
//!
//! The `imap append` command, RFC 3501 `APPEND`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use io_imap::{
    rfc3501::append::ImapMessageAppendOptions,
    types::{IntoStatic, flag::Flag, mailbox::Mailbox},
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    imap::{client::ImapClient, mailbox::arg::MailboxNameArg},
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
    /// This is the raw IMAP API, not the shared
    /// `seen|answered|flagged|draft` enum: a system flag keeps its
    /// backslash, as in `-f '\Seen'`, and a bare word is a custom
    /// keyword, so `-f seen` stores the keyword `seen` and `imap search
    /// --seen` will not match it.
    ///
    /// The shared `message add -f seen` is the enum-mapped behaviour.
    #[arg(short, long, num_args = 0..)]
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

        client.append(
            mailbox,
            message.as_bytes(),
            ImapMessageAppendOptions {
                flags,
                date: None,
                non_sync: false,
            },
        )?;

        printer.out(Message::new("Message successfully saved"))
    }
}
