use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_email::flag::types::Flag;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::shared::{
    client::EmailClient,
    flag::arg::FlagArg,
    message::{
        arg::MessageArg,
        handler::{self, Outcome},
    },
};

/// Add a raw RFC 5322 message to a mailbox.
///
/// The message can be passed as a positional file path, an inline raw
/// string, or piped via stdin (see [`MessageArg`] for resolution
/// order). The destination is resolved through the account's
/// `[mailbox.alias]` map before the backend call. Pass `--send` to
/// also push the message through the account's send path after the
/// append (mirrors `messages send --save <MAILBOX>`).
///
/// IMAP appends via `APPEND` (RFC 3501); JMAP uploads the blob and
/// imports it via `Email/import` (the destination mailbox is
/// resolved by exact-match name); Maildir writes a new file under
/// the target maildir's `cur/` subdir using the standard
/// tmp-then-rename delivery protocol.
#[derive(Debug, Parser)]
pub struct MessageAddCommand {
    /// Destination mailbox name or alias. Mandatory.
    #[arg(long = "mailbox", short = 'm', value_name = "NAME")]
    pub mailbox: String,

    /// Flag(s) to set on the new message. Optional.
    #[arg(long = "flag", short = 'f', value_name = "FLAG", num_args = 0..)]
    pub flag: Vec<FlagArg>,

    /// Send the message after appending it. Combines with the
    /// mandatory `--mailbox` to save-then-send.
    #[arg(long)]
    pub send: bool,

    #[command(flatten)]
    pub message: MessageArg,
}

impl MessageAddCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();
        let flags: Vec<Flag> = self.flag.iter().map(Into::into).collect();
        let outcome = handler::apply(account, client, raw, &flags, Some(&self.mailbox), self.send)?;
        let Outcome::Saved { id, sent } = outcome else {
            unreachable!("--mailbox is mandatory; handler::apply always reports Saved");
        };
        printer.out(MessageAddOutput { id, sent })
    }
}

#[derive(Serialize)]
struct MessageAddOutput {
    id: String,
    sent: bool,
}

impl fmt::Display for MessageAddOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = if self.sent { " and sent" } else { "" };
        write!(f, "Message {} successfully added{suffix}", self.id)
    }
}
