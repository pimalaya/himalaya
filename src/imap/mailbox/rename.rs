use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::rename::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::{
    config::ImapConfig,
    imap::{
        mailbox::arg::{MailboxNameArg, TargetMailboxNameArg},
        stream,
    },
};

/// Rename the given mailbox.
///
/// This command renames an existing mailbox to a new name.
#[derive(Debug, Parser)]
pub struct RenameMailboxCommand {
    #[command(flatten)]
    pub from: MailboxNameArg,

    #[command(flatten)]
    pub to: TargetMailboxNameArg,
}

impl RenameMailboxCommand {
    pub fn exec(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        let (context, mut stream) = stream::connect(config)?;

        let from = self.from.name.try_into()?;
        let to = self.to.name.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapRename::new(context, from, to);

        loop {
            match coroutine.resume(arg.take()) {
                ImapRenameResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapRenameResult::Ok { .. } => break,
                ImapRenameResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully renamed"))
    }
}
