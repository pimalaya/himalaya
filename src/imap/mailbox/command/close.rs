use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::close::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::{config::ImapConfig, imap::stream};

/// Close a mailbox.
///
/// This command first selects the given mailbox, then closes it.
/// CLOSE permanently removes all messages with the \Deleted flag
/// and returns to the authenticated state.
#[derive(Debug, Parser)]
pub struct CloseMailboxCommand;

impl CloseMailboxCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        let (context, mut stream) = stream::connect(config)?;

        let mut arg = None;
        let mut close_coroutine = ImapClose::new(context);

        loop {
            match close_coroutine.resume(arg.take()) {
                ImapCloseResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapCloseResult::Ok { .. } => break,
                ImapCloseResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully closed"))
    }
}
