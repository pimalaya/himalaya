//! # Message move
//!
//! The `message move` command, moving messages between two mailboxes of
//! one account.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    error::{ItemError, partial_when, write_item_errors},
    unified::{
        client::EmailClient, flag::arg::MessageIdsArg, mailbox::arg::resolve_mailbox_or_default,
    },
};

/// Move messages from one mailbox to another within the active account.
///
/// Both mailboxes are resolved through the account's aliases. Moving
/// across accounts or backends is out of scope.
///
/// An id the mailbox does not hold is an entry of `errors`, the other
/// messages still acted on; the command then exits 3.
#[derive(Debug, Parser)]
pub struct MessageMoveCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    /// Source mailbox name, alias, role or backend-native id, the inbox
    /// when omitted: the `inbox` alias, else the backend's inbox role.
    #[arg(long = "from", short = 'f', value_name = "NAME")]
    pub from: Option<String>,
    /// Destination mailbox name, alias, role or backend-native id.
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
        let ids = self.ids.ids();
        let outcome = client.move_messages(&from, &to, &ids)?;

        let output = MessageMoveOutput {
            count: outcome.value,
            queued: client.queues_writes(),
            errors: outcome.errors,
        };

        printer.out(&output)?;
        partial_when(&output.errors)
    }
}

/// The `message move` output: how many messages were moved.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageMoveOutput {
    /// How many of the given messages were moved.
    count: usize,
    /// Whether a sync engine applies the write later rather than now.
    queued: bool,
    /// The error of each message not acted on, in the order given.
    errors: Vec<ItemError>,
}

impl fmt::Display for MessageMoveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.count;

        if self.queued {
            writeln!(f, "{count} message(s) queued for moving")?;
        } else {
            writeln!(f, "{count} message(s) successfully moved")?;
        }

        write_item_errors(f, &self.errors)
    }
}
