use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{expunge::*, select::*, store::*},
    types::{
        flag::{Flag, StoreType},
        sequence::SequenceSet,
    },
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::{config::ImapConfig, imap::mailbox::arg::name::MailboxNameArg, imap::stream};

/// Purge the given mailbox.
///
/// All emails from the given mailbox are definitely deleted. The
/// purged mailbox will remain empty after execution of the command.
#[derive(Debug, Parser)]
pub struct PurgeMailboxCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
}

impl PurgeMailboxCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        let (context, mut stream) = stream::connect(config)?;

        let mailbox = self.mailbox.name.try_into()?;

        // First, select the mailbox
        let mut arg = None;
        let mut coroutine = ImapSelect::new(context, mailbox);

        let context = loop {
            match coroutine.resume(arg.take()) {
                ImapSelectResult::Io(io) => arg = Some(handle(&mut stream, io)?),
                ImapSelectResult::Ok { context, .. } => break context,
                ImapSelectResult::Err { err, .. } => bail!(err),
            }
        };

        // Then, mark all messages as deleted (1:*)
        let mut arg = None;
        let sequence_set: SequenceSet = "1:*".try_into()?;
        let mut coroutine = ImapStoreSilent::new(
            context,
            sequence_set,
            StoreType::Add,
            vec![Flag::Deleted],
            false,
        );

        let context = loop {
            match coroutine.resume(arg.take()) {
                ImapStoreSilentResult::Io(io) => arg = Some(handle(&mut stream, io)?),
                ImapStoreSilentResult::Ok { context, .. } => break context,
                ImapStoreSilentResult::Err { err, .. } => bail!(err),
            }
        };

        // Finally, expunge the mailbox
        let mut arg = None;
        let mut coroutine = ImapExpunge::new(context);

        loop {
            match coroutine.resume(arg.take()) {
                ImapExpungeResult::Io(io) => arg = Some(handle(&mut stream, io)?),
                ImapExpungeResult::Ok { .. } => break,
                ImapExpungeResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully purged"))
    }
}
