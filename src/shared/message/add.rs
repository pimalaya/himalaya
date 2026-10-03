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

use crate::shared::note::Noted;
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
    /// Send the message too, before appending it, as `message send
    /// --save` does.
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
        let Outcome::Saved { id, sent, queued } = outcome else {
            unreachable!("--mailbox is mandatory; handler::apply always reports Saved");
        };
        printer.out(Noted {
            output: MessageAddOutput {
                id,
                sent,
                queue_id: queued,
            },
            notes: client.take_notes(),
        })
    }
}

/// The `message add` output, naming the message that was appended.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageAddOutput {
    id: Option<String>,
    sent: bool,
    /// The queue row id of a send deferred to the store's owner.
    #[serde(skip_serializing_if = "Option::is_none")]
    queue_id: Option<i64>,
}

impl fmt::Display for MessageAddOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = match (self.sent, self.queue_id) {
            (true, Some(_)) => " and queued for sending",
            (true, None) => " and sent",
            (false, _) => "",
        };
        match &self.id {
            Some(id) => write!(f, "Message {id} successfully added{suffix}"),
            None => write!(f, "Message successfully added{suffix}, id unavailable"),
        }
    }
}
