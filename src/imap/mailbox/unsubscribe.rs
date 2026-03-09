use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::unsubscribe::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg, stream};

/// Unsubscribe from the given mailbox.
///
/// This command unsubscribes from a mailbox, removing it from the
/// list of subscribed mailboxes.
#[derive(Debug, Parser)]
pub struct UnsubscribeMailboxCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
}

impl UnsubscribeMailboxCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let mailbox = self.mailbox.name.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapUnsubscribe::new(context, mailbox);

        loop {
            match coroutine.resume(arg.take()) {
                ImapUnsubscribeResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapUnsubscribeResult::Ok { .. } => break,
                ImapUnsubscribeResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully unsubscribed"))
    }
}
