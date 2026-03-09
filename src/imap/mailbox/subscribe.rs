use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::subscribe::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg, stream};

/// Subscribe to the given mailbox.
///
/// This command subscribes to a mailbox, making it appear in the
/// list of subscribed mailboxes.
#[derive(Debug, Parser)]
pub struct SubscribeMailboxCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
}

impl SubscribeMailboxCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let mailbox = self.mailbox.name.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapSubscribe::new(context, mailbox);

        loop {
            match coroutine.resume(arg.take()) {
                ImapSubscribeResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapSubscribeResult::Ok { .. } => break,
                ImapSubscribeResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully subscribed"))
    }
}
