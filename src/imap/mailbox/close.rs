use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::close::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, stream};

/// Close the current, selected mailbox.
///
/// This command also expunges the current mailbox and returns to the
/// authenticated state.
///
/// NOTE: Since a selected mailbox is required, this command only
/// works for stateful IMAP sessions. See:
///
/// https://github.com/pimalaya/sirup
#[derive(Debug, Parser)]
pub struct CloseMailboxCommand;

impl CloseMailboxCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let mut arg = None;
        let mut close_coroutine = ImapClose::new(context);

        loop {
            match close_coroutine.resume(arg.take()) {
                ImapCloseResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapCloseResult::Ok { .. } => break,
                ImapCloseResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully closed"))
    }
}
