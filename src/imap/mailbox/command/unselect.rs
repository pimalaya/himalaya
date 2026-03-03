use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::{select::*, unselect::*};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::{config::ImapConfig, imap::mailbox::arg::name::MailboxNameArg, imap::stream};

/// Unselect a mailbox.
///
/// This command first selects the given mailbox, then unselects it.
/// Unlike CLOSE, UNSELECT does not expunge deleted messages.
#[derive(Debug, Parser)]
pub struct UnselectMailboxCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,
}

impl UnselectMailboxCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        let (context, mut stream) = stream::connect(config)?;

        let mailbox = self.mailbox.name.try_into()?;

        // First SELECT the mailbox
        let mut arg = None;
        let mut select_coroutine = ImapSelect::new(context, mailbox);

        let context = loop {
            match select_coroutine.resume(arg.take()) {
                ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapSelectResult::Ok { context, .. } => break context,
                ImapSelectResult::Err { err, .. } => bail!(err),
            }
        };

        // Then UNSELECT
        let mut arg = None;
        let mut unselect_coroutine = ImapUnselect::new(context);

        loop {
            match unselect_coroutine.resume(arg.take()) {
                ImapUnselectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapUnselectResult::Ok { .. } => break,
                ImapUnselectResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully unselected"))
    }
}
