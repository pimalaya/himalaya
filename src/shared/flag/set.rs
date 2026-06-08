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

/// Replace flag(s) of message(s) for the active account.
#[derive(Debug, Parser)]
pub struct FlagSetCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    #[command(flatten)]
    pub message_ids: MessageIdsArg,
    #[command(flatten)]
    pub flags: FlagsArg,
}

impl FlagSetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account)?;
        let ids: Vec<&str> = self.message_ids.inner.iter().map(String::as_str).collect();
        let flags: Vec<Flag> = self.flags.inner.iter().map(Into::into).collect();

        client.store_flags(&mailbox, &ids, &flags, FlagOp::Set)?;

        let flags: Vec<String> = self.flags.inner.iter().map(ToString::to_string).collect();
        printer.out(SetFlags { flags })
    }
}

#[derive(Debug, Serialize)]
struct SetFlags {
    flags: Vec<String>,
}

impl fmt::Display for SetFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Successfully set flags: {}", self.flags.join(", "))
    }
}
