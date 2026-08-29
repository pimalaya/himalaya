//! # Message add
//!
//! The `message add` command, appending a raw RFC 5322 message to a
//! mailbox.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::flag::Flag,
    shared::{
        client::EmailClient,
        flag::arg::FlagArg,
        message::{
            arg::MessageArg,
            handler::{self, Outcome},
        },
    },
};

/// Add a raw RFC 5322 message to a mailbox.
///
/// The message comes from a file path, an inline string or piped standard
/// input, and the mailbox is resolved through the account's aliases.
#[derive(Debug, Parser)]
pub struct MessageAddCommand {
    /// Destination mailbox name or alias.
    #[arg(long = "mailbox", short = 'm', value_name = "NAME")]
    pub mailbox: String,
    /// Flags to set on the new message.
    #[arg(long = "flag", short = 'f', value_name = "FLAG", num_args = 0..)]
    pub flag: Vec<FlagArg>,
    /// Send the message once appended, which is `message send --save`
    /// the other way round.
    #[arg(long)]
    pub send: bool,
    #[command(flatten)]
    pub message: MessageArg,
}

impl MessageAddCommand {
    /// Appends the message, sends it when asked, and reports its new id.
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

/// The `message add` output, naming the message that was appended.
#[derive(Serialize, JsonSchema)]
pub(crate) struct MessageAddOutput {
    id: String,
    sent: bool,
}

impl fmt::Display for MessageAddOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = if self.sent { " and sent" } else { "" };
        write!(f, "Message {} successfully added{suffix}", self.id)
    }
}
