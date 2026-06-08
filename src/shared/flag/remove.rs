use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_email::flag::types::{Flag, FlagOp};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::shared::{
    client::EmailClient,
    flag::arg::{FlagsArg, MessageIdsArg},
    mailbox::arg::MailboxArg,
};

/// Remove flag(s) from message(s) for the active account.
#[derive(Debug, Parser)]
pub struct FlagRemoveCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    #[command(flatten)]
    pub message_ids: MessageIdsArg,
    #[command(flatten)]
    pub flags: FlagsArg,
}

impl FlagRemoveCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account)?;
        let ids: Vec<&str> = self.message_ids.inner.iter().map(String::as_str).collect();
        let flags: Vec<Flag> = self.flags.inner.iter().map(Into::into).collect();

        client.store_flags(&mailbox, &ids, &flags, FlagOp::Remove)?;

        let flags: Vec<String> = self.flags.inner.iter().map(ToString::to_string).collect();
        printer.out(RemovedFlags { flags })
    }
}

#[derive(Debug, Serialize)]
struct RemovedFlags {
    flags: Vec<String>,
}

impl fmt::Display for RemovedFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Successfully removed flags: {}", self.flags.join(", "))
    }
}
