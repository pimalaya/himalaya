use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::unsubscribe::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::{config::ImapConfig, imap::mailbox::arg::name::MailboxNameArg, imap::stream};

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
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        let (context, mut stream) = stream::connect(config)?;

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
