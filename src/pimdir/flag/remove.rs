//! # pimdir flag remove
//!
//! The `pimdir flag remove` command, staging flags removed from the
//! existing set of one or more messages.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    email::flag::FlagOp,
    pimdir::{
        client::PimdirClient,
        flag::{PimdirFlagsStaged, RawFlagsArg, message_ids},
    },
    shared::{flag::arg::MessageIdsArg, mailbox::arg::MailboxArg},
};

/// Stage flags removed from messages, keeping the others.
///
/// Every message is checked against the store's sources before the first row
/// is queued, so a message the check refuses stages nothing for the others.
/// The sync engine pushes the change on its next run.
#[derive(Debug, Parser)]
pub struct PimdirFlagRemoveCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    #[command(flatten)]
    pub message_ids: MessageIdsArg,
    #[command(flatten)]
    pub flags: RawFlagsArg,
}

impl PimdirFlagRemoveCommand {
    /// Stages the removals and prints the row each message waits in.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let flags = self.flags.parse()?;
        let mailbox = self.mailbox.resolve(account);
        let ids = message_ids(&self.message_ids)?;
        let staged = client.stage_flags(&mailbox, &ids, &flags, FlagOp::Remove)?;
        printer.out(PimdirFlagsStaged::found(staged)?)
    }
}
