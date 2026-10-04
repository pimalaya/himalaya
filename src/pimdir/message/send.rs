//! # pimdir message send
//!
//! The `pimdir message send` command, queueing a message for the store's
//! owner to send and printing its queue row.

use std::fmt;

use anyhow::{Context, Result};
use clap::Parser;
use pimalaya_cli::{printer::Printer, table::sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::{
        flag::{Flag, IanaFlag},
        mailbox::MailboxRole,
    },
    pimdir::client::PimdirClient,
    shared::message::arg::MessageArg,
};

/// Queue a message for sending, printing its queue row.
///
/// The message is sent as `message send` sends it: one `submit` row the
/// sync engine performs, anchored on the `--save` mailbox or the `sent`
/// alias. The row id is what `pimdir queue cancel` takes while the message
/// has not left.
#[derive(Debug, Parser)]
pub struct PimdirMessageSendCommand {
    /// Append a copy of the sent message to this mailbox name, alias or
    /// role, overriding `message.send.save-copy`.
    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,
    /// Skip the copy `message.send.save-copy` configures.
    #[arg(long, conflicts_with = "save")]
    pub no_save: bool,
    #[command(flatten)]
    pub message: MessageArg,
}

impl PimdirMessageSendCommand {
    /// Queues the send, and the copy when the owner cannot carry it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();
        let save = account
            .resolve_save(self.save.as_deref(), self.no_save, true)
            .map(|name| account.resolve_mailbox(name));
        let anchor = save.or_else(|| {
            account
                .mailbox_alias
                .get(MailboxRole::Sent.as_str())
                .map(String::as_str)
        });

        let sent = client.send_message(anchor, raw.clone(), save.is_some())?;

        // NOTE: an owner declaring no capabilities ignores the copy in the
        // intent, so it is queued beside the send, as `message send` does.
        let copy_queue_id = match save {
            Some(mailbox) if !sent.carried => {
                let seen = Flag::from_iana(IanaFlag::Seen);
                let copy = client.add_message(mailbox, &[seen], raw).with_context(|| {
                    format!("Message queued for sending, but queueing its copy to {mailbox} failed")
                })?;
                Some(copy.queue_id)
            }
            _ => None,
        };

        printer.out(PimdirMessageSent {
            queue_id: sent.staged.queue_id,
            message_id: sent.staged.message_id,
            copy: sent.carried,
            copy_queue_id,
        })
    }
}

/// The `pimdir message send` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirMessageSent {
    /// The `submit` row the message waits in until the owner sends it.
    pub queue_id: i64,
    /// The message's bare `Message-ID`.
    pub message_id: String,
    /// Whether the `submit` row carries the copy, filed once sent.
    pub copy: bool,
    /// The row of the copy queued beside the send, for an owner that
    /// cannot carry it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy_queue_id: Option<i64>,
}

impl fmt::Display for PimdirMessageSent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Message {} queued for sending as row {}",
            sanitize(&self.message_id),
            self.queue_id
        )?;
        if let Some(copy) = self.copy_queue_id {
            write!(f, ", its copy as row {copy}")?;
        }
        Ok(())
    }
}
