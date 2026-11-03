//! # Message move
//!
//! The `message move` command, moving messages between two mailboxes of
//! one account.

use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    shared::{
        client::EmailClient, flag::arg::MessageIdsArg, mailbox::arg::resolve_mailbox_or_default,
    },
};

/// Move messages from one mailbox to another within the active account.
///
/// Both mailboxes are resolved through the account's aliases. Moving
/// across accounts or backends is out of scope.
#[derive(Debug, Parser)]
pub struct MessageMoveCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    /// Source mailbox name or alias, the `inbox` alias when omitted.
    ///
    /// The command errors when neither is given, the shared layer having
    /// no way to guess a backend's inbox id.
    #[arg(long = "from", short = 'f', value_name = "NAME")]
    pub from: Option<String>,
    /// Destination mailbox name or alias.
    #[arg(long = "to", short = 't', value_name = "NAME")]
    pub to: String,
}

impl MessageMoveCommand {
    /// Moves the messages and reports how many landed.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let from = resolve_mailbox_or_default(account, self.from.as_deref());
        let to = account.resolve_mailbox(&self.to).to_owned();
        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();
        let count = client.move_messages(&from, &to, &ids)?;
        if count == 0 {
            bail!("No message moved: no id matched in the source mailbox");
        }

        printer.out(MessageMoveOutput {
            count,
            queued: client.queues_writes(),
        })
    }
}

/// The `message move` output: how many messages were moved.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageMoveOutput {
    /// How many of the given messages were found and moved.
    count: usize,
    /// Whether a sync engine applies the write later rather than now.
    queued: bool,
}

impl fmt::Display for MessageMoveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.count;

        if self.queued {
            return writeln!(f, "{count} message(s) queued for moving");
        }

        writeln!(f, "{count} message(s) successfully moved")
    }
}
