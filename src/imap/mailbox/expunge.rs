use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::{expunge::*, select::*};
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameArg, MailboxNoSelectFlag},
};

/// Expunge the given mailbox.
///
/// All envelopes with the \Deleted flag will be definitely removed
/// from the given mailbox.
#[derive(Debug, Parser)]
pub struct ImapMailboxExpungeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,
}

impl ImapMailboxExpungeCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            let mut arg = None;
            let mut coroutine = ImapMailboxSelect::new(imap.context, mailbox);

            imap.context = loop {
                match coroutine.resume(arg.take()) {
                    ImapMailboxSelectResult::Io { input } => {
                        arg = Some(handle(&mut imap.stream, input)?)
                    }
                    ImapMailboxSelectResult::Ok { context, .. } => break context,
                    ImapMailboxSelectResult::Err { err, .. } => bail!(err),
                }
            };
        }

        let mut arg = None;
        let mut coroutine = ImapMailboxExpunge::new(imap.context);

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxExpungeResult::Io { input } => {
                    arg = Some(handle(&mut imap.stream, input)?)
                }
                ImapMailboxExpungeResult::Ok { .. } => break,
                ImapMailboxExpungeResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully expunged"))
    }
}
