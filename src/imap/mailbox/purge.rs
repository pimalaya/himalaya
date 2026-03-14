use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{expunge::*, select::*, store::*},
    types::flag::{Flag, StoreType},
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameArg, MailboxSelectFlag},
};

/// Shortcut for marking as deleted all envelopes then expunging the
/// given mailbox.
///
/// All emails from the given mailbox are definitely deleted. The
/// purged mailbox will remain empty after execution of the command.
#[derive(Debug, Parser)]
pub struct PurgeMailboxCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
    #[command(flatten)]
    pub select: MailboxSelectFlag,
}

impl PurgeMailboxCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox.name.try_into()?;

        if self.select.r#true {
            let mut arg = None;
            let mut coroutine = ImapSelect::new(imap.context, mailbox);

            imap.context = loop {
                match coroutine.resume(arg.take()) {
                    ImapSelectResult::Io { io } => arg = Some(handle(&mut imap.stream, io)?),
                    ImapSelectResult::Ok { context, .. } => break context,
                    ImapSelectResult::Err { err, .. } => bail!(err),
                }
            };
        }

        let mut arg = None;
        let mut coroutine = ImapStoreSilent::new(
            imap.context,
            "1:*".try_into()?,
            StoreType::Add,
            vec![Flag::Deleted],
            false,
        );

        imap.context = loop {
            match coroutine.resume(arg.take()) {
                ImapStoreSilentResult::Io { io } => arg = Some(handle(&mut imap.stream, io)?),
                ImapStoreSilentResult::Ok { context, .. } => break context,
                ImapStoreSilentResult::Err { err, .. } => bail!(err),
            }
        };

        let mut arg = None;
        let mut coroutine = ImapExpunge::new(imap.context);

        loop {
            match coroutine.resume(arg.take()) {
                ImapExpungeResult::Io { io } => arg = Some(handle(&mut imap.stream, io)?),
                ImapExpungeResult::Ok { .. } => break,
                ImapExpungeResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully purged"))
    }
}
