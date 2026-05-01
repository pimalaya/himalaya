use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::{expunge::*, select::*};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameArg, MailboxNoSelectFlag},
};

const READ_BUFFER_SIZE: usize = 16 * 1024;

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

        let mut buf = [0u8; READ_BUFFER_SIZE];

        if !self.mailbox_no_select.inner {
            let mut coroutine = ImapMailboxSelect::new(imap.context, mailbox);
            let mut arg: Option<&[u8]> = None;

            imap.context = loop {
                match coroutine.resume(arg.take()) {
                    ImapMailboxSelectResult::Ok { context, .. } => break context,
                    ImapMailboxSelectResult::WantsRead => {
                        let n = imap.stream.read(&mut buf)?;
                        arg = Some(&buf[..n]);
                    }
                    ImapMailboxSelectResult::WantsWrite(bytes) => {
                        imap.stream.write_all(&bytes)?;
                        arg = None;
                    }
                    ImapMailboxSelectResult::Err { err, .. } => bail!("{err}"),
                }
            };
        }

        let mut coroutine = ImapMailboxExpunge::new(imap.context);
        let mut arg: Option<&[u8]> = None;

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxExpungeResult::Ok { .. } => break,
                ImapMailboxExpungeResult::WantsRead => {
                    let n = imap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                ImapMailboxExpungeResult::WantsWrite(bytes) => {
                    imap.stream.write_all(&bytes)?;
                    arg = None;
                }
                ImapMailboxExpungeResult::Err { err, .. } => bail!("{err}"),
            }
        }

        printer.out(Message::new("Mailbox successfully expunged"))
    }
}
