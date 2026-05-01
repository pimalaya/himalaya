use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3691::unselect::*;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::account::ImapAccount;

const READ_BUFFER_SIZE: usize = 16 * 1024;

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

        let mut coroutine = ImapMailboxUnselect::new(imap.context);
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxUnselectResult::Ok(_) => break,
                ImapMailboxUnselectResult::WantsRead => {
                    let n = imap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                ImapMailboxUnselectResult::WantsWrite(bytes) => {
                    imap.stream.write_all(&bytes)?;
                    arg = None;
                }
                ImapMailboxUnselectResult::Err { err, .. } => bail!("{err}"),
            }
        }

        printer.out(Message::new("Mailbox successfully unselected"))
    }
}
