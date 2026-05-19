use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_email::flag::Flag;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::shared::{
    client::EmailClient,
    flags::arg::{FlagsArg, MessageIdsArg},
    mailboxes::arg::MailboxArg,
};

/// Add flag(s) to message(s) for the active account.
#[derive(Debug, Parser)]
pub struct FlagAddCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    #[command(flatten)]
    pub message_ids: MessageIdsArg,
    #[command(flatten)]
    pub flags: FlagsArg,
}

impl FlagAddCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let mailbox = self.mailbox.resolve(&client.account)?;
        let ids: Vec<&str> = self.message_ids.inner.iter().map(String::as_str).collect();
        let flags: Vec<Flag> = self.flags.inner.iter().map(Into::into).collect();

        client.add_flags(&mailbox, &ids, &flags)?;

        let flags: Vec<String> = self.flags.inner.iter().map(ToString::to_string).collect();
        printer.out(AddedFlags { flags })
    }
}

#[derive(Debug, Serialize)]
struct AddedFlags {
    flags: Vec<String>,
}

impl fmt::Display for AddedFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Successfully added flags: {}", self.flags.join(", "))
    }
}
