use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::select::*;
use io_socket::runtimes::std_stream::handle;
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
pub struct ImapMailboxSelectCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxSelectCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapMailboxSelect::new(imap.context, mailbox);

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxSelectResult::Io { input } => {
                    arg = Some(handle(&mut imap.stream, input)?)
                }
                ImapMailboxSelectResult::Ok { .. } => break,
                ImapMailboxSelectResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully selected"))
    }
}
