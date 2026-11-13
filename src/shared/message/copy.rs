//! # Message copy
//!
//! The `message copy` command, copying messages between two mailboxes of
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

/// Copy messages from one mailbox to another within the active account.
///
/// Both mailboxes are resolved through the account's aliases. Copying
/// across accounts or backends is out of scope.
#[derive(Debug, Parser)]
pub struct MessageCopyCommand {
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

impl MessageCopyCommand {
    /// Copies the messages and reports how many landed.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let from = resolve_mailbox_or_default(account, self.from.as_deref());
        let to = account.resolve_mailbox(&self.to).to_owned();
        let ids = self.ids.ids();
        let count = client.copy_messages(&from, &to, &ids)?;
        if count == 0 {
            bail!("No message copied: no id matched in the source mailbox");
        }

        printer.out(MessageCopyOutput {
            count,
            queued: client.queues_writes(),
        })
    }
}

/// The `message copy` output: how many messages were copied.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageCopyOutput {
    /// How many of the given messages were found and copied.
    count: usize,
    /// Whether a sync engine applies the write later rather than now.
    queued: bool,
}

impl fmt::Display for MessageCopyOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.count;

        if self.queued {
            return writeln!(f, "{count} message(s) queued for copying");
        }

        writeln!(f, "{count} message(s) successfully copied")
    }
}
