use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::close::*;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::account::ImapAccount;

const READ_BUFFER_SIZE: usize = 16 * 1024;

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

        let mut close_coroutine = ImapMailboxClose::new(imap.context);
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        loop {
            match close_coroutine.resume(arg.take()) {
                ImapMailboxCloseResult::Ok(_) => break,
                ImapMailboxCloseResult::WantsRead => {
                    let n = imap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                ImapMailboxCloseResult::WantsWrite(bytes) => {
                    imap.stream.write_all(&bytes)?;
                    arg = None;
                }
                ImapMailboxCloseResult::Err { err, .. } => bail!("{err}"),
            }
        }

        printer.out(Message::new("Mailbox successfully closed"))
    }
}
