use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::create::*;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameArg};

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Create the given mailbox.
///
/// This command allows you to create a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct ImapMailboxCreateCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;

        let mailbox = self.mailbox_name.inner.try_into()?;

        let mut coroutine = ImapMailboxCreate::new(imap.context, mailbox);
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxCreateResult::Ok(_) => break,
                ImapMailboxCreateResult::WantsRead => {
                    let n = imap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                ImapMailboxCreateResult::WantsWrite(bytes) => {
                    imap.stream.write_all(&bytes)?;
                    arg = None;
                }
                ImapMailboxCreateResult::Err { err, .. } => bail!("{err}"),
            }
        }

        printer.out(Message::new("Mailbox successfully created"))
    }
}
