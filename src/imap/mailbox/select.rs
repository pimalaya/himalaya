use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::select::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

/// Select the given mailbox.
///
/// This command permanently removes all messages with the \Deleted
/// flag and returns to the authenticated state.
///
/// NOTE: This command only works for stateful IMAP sessions. See:
///
/// https://github.com/pimalaya/sirup
#[derive(Debug, Parser)]
pub struct SelectMailboxCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
}

impl SelectMailboxCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox.name.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapSelect::new(imap.context, mailbox);

        loop {
            match coroutine.resume(arg.take()) {
                ImapSelectResult::Io { io } => arg = Some(handle(&mut imap.stream, io)?),
                ImapSelectResult::Ok { .. } => break,
                ImapSelectResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully selected"))
    }
}
