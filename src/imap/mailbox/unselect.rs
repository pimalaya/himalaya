use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3691::unselect::*;
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::account::ImapAccount;

/// Unselect a current, selected mailbox.
///
/// Unlike CLOSE, UNSELECT does not expunge deleted messages.
///
/// NOTE: Since a selected mailbox is required, this command only
/// works for stateful IMAP sessions. See:
///
/// https://github.com/pimalaya/sirup
#[derive(Debug, Parser)]
pub struct ImapMailboxUnselectCommand;

impl ImapMailboxUnselectCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mut arg = None;
        let mut coroutine = ImapMailboxUnselect::new(imap.context);

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxUnselectResult::Io { input } => {
                    arg = Some(handle(&mut imap.stream, input)?)
                }
                ImapMailboxUnselectResult::Ok { .. } => break,
                ImapMailboxUnselectResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully unselected"))
    }
}
