use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::unsubscribe::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

/// Unsubscribe from the given mailbox.
///
/// This command unsubscribes from a mailbox, removing it from the
/// list of subscribed mailboxes.
#[derive(Debug, Parser)]
pub struct UnsubscribeMailboxCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl UnsubscribeMailboxCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapUnsubscribe::new(imap.context, mailbox);

        loop {
            match coroutine.resume(arg.take()) {
                ImapUnsubscribeResult::Io { io } => arg = Some(handle(&mut imap.stream, io)?),
                ImapUnsubscribeResult::Ok { .. } => break,
                ImapUnsubscribeResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully unsubscribed"))
    }
}
