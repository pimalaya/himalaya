use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::subscribe::*;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Subscribe to the given mailbox.
///
/// This command subscribes to a mailbox, making it appear in the
/// list of subscribed mailboxes.
#[derive(Debug, Parser)]
pub struct ImapMailboxSubscribeCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxSubscribeCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        let mut coroutine = ImapMailboxSubscribe::new(imap.context, mailbox);
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxSubscribeResult::Ok(_) => break,
                ImapMailboxSubscribeResult::WantsRead => {
                    let n = imap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                ImapMailboxSubscribeResult::WantsWrite(bytes) => {
                    imap.stream.write_all(&bytes)?;
                    arg = None;
                }
                ImapMailboxSubscribeResult::Err { err, .. } => bail!("{err}"),
            }
        }

        printer.out(Message::new("Mailbox successfully subscribed"))
    }
}
