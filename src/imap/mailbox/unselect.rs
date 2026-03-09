use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::unselect::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, stream};

/// Unselect a current, selected mailbox.
///
/// Unlike CLOSE, UNSELECT does not expunge deleted messages.
///
/// NOTE: Since a selected mailbox is required, this command only
/// works for stateful IMAP sessions. See:
///
/// https://github.com/pimalaya/sirup
#[derive(Debug, Parser)]
pub struct UnselectMailboxCommand;

impl UnselectMailboxCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let mut arg = None;
        let mut unselect_coroutine = ImapUnselect::new(context);

        loop {
            match unselect_coroutine.resume(arg.take()) {
                ImapUnselectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapUnselectResult::Ok { .. } => break,
                ImapUnselectResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully unselected"))
    }
}
