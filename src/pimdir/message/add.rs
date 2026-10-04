//! # pimdir message add
//!
//! The `pimdir message add` command, staging a message in a mailbox and
//! printing its queue row.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{printer::Printer, table::sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::flag::Flag,
    pimdir::client::PimdirClient,
    shared::{flag::arg::FlagArg, message::arg::MessageArg},
};

/// Stage a raw message in a mailbox, printing its queue row.
///
/// The message is added as `message add` adds it. It has no id until the
/// sync engine applies the row; the row id follows it until then, and the
/// `Message-ID` it is filed under after.
#[derive(Debug, Parser)]
pub struct PimdirMessageAddCommand {
    /// Destination mailbox name or alias.
    #[arg(long = "mailbox", short = 'm', value_name = "NAME")]
    pub mailbox: String,
    /// Flags to set on the new message.
    #[arg(long = "flag", short = 'f', value_name = "FLAG", num_args = 0..)]
    pub flag: Vec<FlagArg>,
    #[command(flatten)]
    pub message: MessageArg,
}

impl PimdirMessageAddCommand {
    /// Stages the message and prints the row it waits in.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();
        let flags: Vec<Flag> = self.flag.iter().map(Into::into).collect();
        let mailbox = account.resolve_mailbox(&self.mailbox);
        let staged = client.add_message(mailbox, &flags, raw)?;

        printer.out(PimdirMessageAdded {
            queue_id: staged.queue_id,
            message_id: staged.message_id,
        })
    }
}

/// The `pimdir message add` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMessageAdded {
    /// The queue row the message waits in.
    pub queue_id: i64,
    /// The bare `Message-ID` it is filed under once applied.
    pub message_id: String,
}

impl fmt::Display for PimdirMessageAdded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Message {} queued as row {}",
            sanitize(&self.message_id),
            self.queue_id
        )
    }
}
