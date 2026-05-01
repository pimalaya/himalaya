use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::rename::*;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameArg, TargetMailboxNameArg},
};

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Rename the given mailbox.
///
/// This command renames an existing mailbox to a new name.
#[derive(Debug, Parser)]
pub struct ImapMailboxRenameCommand {
    #[command(flatten)]
    pub mailbox_source_name: MailboxNameArg,
    #[command(flatten)]
    pub mailbox_dest_name: TargetMailboxNameArg,
}

impl ImapMailboxRenameCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let from = self.mailbox_source_name.inner.try_into()?;
        let to = self.mailbox_dest_name.inner.try_into()?;

        let mut coroutine = ImapMailboxRename::new(imap.context, from, to);
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxRenameResult::Ok(_) => break,
                ImapMailboxRenameResult::WantsRead => {
                    let n = imap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                ImapMailboxRenameResult::WantsWrite(bytes) => {
                    imap.stream.write_all(&bytes)?;
                    arg = None;
                }
                ImapMailboxRenameResult::Err { err, .. } => bail!("{err}"),
            }
        }

        printer.out(Message::new("Mailbox successfully renamed"))
    }
}
