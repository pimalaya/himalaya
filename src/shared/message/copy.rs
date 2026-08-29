//! # Message copy
//!
//! The `message copy` command, copying messages between two mailboxes of
//! one account.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

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

impl MessageCopyCommand {
    /// Copies the messages and reports how many landed.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let from = resolve_mailbox_or_default(account, self.from.as_deref())?;
        let to = account.resolve_mailbox(&self.to).to_owned();
        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();
        let count = client.copy_messages(&from, &to, &ids)?;
        let message = match count {
            0 => "No message copied: no id matched in the source mailbox".to_string(),
            1 => "1 message successfully copied".to_string(),
            n => format!("{n} messages successfully copied"),
        };
        printer.out(Message::new(message))
    }
}
