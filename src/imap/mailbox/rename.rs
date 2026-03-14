use anyhow::{bail, Result};
use clap::Parser;
use io_imap::coroutines::rename::*;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameArg, TargetMailboxNameArg},
};

/// Rename the given mailbox.
///
/// This command renames an existing mailbox to a new name.
#[derive(Debug, Parser)]
pub struct RenameMailboxCommand {
    #[command(flatten)]
    pub mailbox_source_name: MailboxNameArg,
    #[command(flatten)]
    pub mailbox_dest_name: TargetMailboxNameArg,
}

impl RenameMailboxCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut imap = account.new_imap_session()?;
        let from = self.mailbox_source_name.inner.try_into()?;
        let to = self.mailbox_dest_name.inner.try_into()?;

        let mut arg = None;
        let mut coroutine = ImapRename::new(imap.context, from, to);

        loop {
            match coroutine.resume(arg.take()) {
                ImapRenameResult::Io { io } => arg = Some(handle(&mut imap.stream, io)?),
                ImapRenameResult::Ok { .. } => break,
                ImapRenameResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully renamed"))
    }
}
