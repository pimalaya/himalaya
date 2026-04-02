use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::close::*;
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::account::ImapAccount;

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
pub struct ImapMailboxCloseCommand;

impl ImapMailboxCloseCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;

        let mut arg = None;
        let mut close_coroutine = ImapMailboxClose::new(imap.context);

        loop {
            match close_coroutine.resume(arg.take()) {
                ImapMailboxCloseResult::Io { input } => {
                    arg = Some(handle(&mut imap.stream, input)?)
                }
                ImapMailboxCloseResult::Ok { .. } => break,
                ImapMailboxCloseResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully closed"))
    }
}
