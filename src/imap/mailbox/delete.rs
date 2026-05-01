use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::delete::*;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Delete the given mailbox.
///
/// All emails from the given mailbox are definitely deleted. The
/// mailbox is also deleted after execution of the command.
#[derive(Debug, Parser)]
pub struct ImapMailboxDeleteCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        let mut coroutine = ImapMailboxDelete::new(imap.context, mailbox);
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxDeleteResult::Ok(_) => break,
                ImapMailboxDeleteResult::WantsRead => {
                    let n = imap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                ImapMailboxDeleteResult::WantsWrite(bytes) => {
                    imap.stream.write_all(&bytes)?;
                    arg = None;
                }
                ImapMailboxDeleteResult::Err { err, .. } => bail!("{err}"),
            }
        }

        printer.out(Message::new("Mailbox successfully deleted"))
    }
}
