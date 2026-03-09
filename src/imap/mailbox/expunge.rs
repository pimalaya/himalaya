use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::{expunge::*, select::*};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameArg, MailboxSelectFlag},
    stream,
};

/// Expunge the given mailbox.
///
/// All envelopes with the \Deleted flag will be definitely removed
/// from the given mailbox.
#[derive(Debug, Parser)]
pub struct ExpungeMailboxCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
    #[command(flatten)]
    pub select: MailboxSelectFlag,
}

impl ExpungeMailboxCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (mut context, mut stream) = stream::connect(account.backend)?;

        let mailbox = self.mailbox.name.try_into()?;

        if self.select.r#true {
            let mut arg = None;
            let mut coroutine = ImapSelect::new(context, mailbox);

            context = loop {
                match coroutine.resume(arg.take()) {
                    ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                    ImapSelectResult::Ok { context, .. } => break context,
                    ImapSelectResult::Err { err, .. } => bail!(err),
                }
            };
        }

        let mut arg = None;
        let mut coroutine = ImapExpunge::new(context);

        loop {
            match coroutine.resume(arg.take()) {
                ImapExpungeResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapExpungeResult::Ok { .. } => break,
                ImapExpungeResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully expunged"))
    }
}
