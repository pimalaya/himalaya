use anyhow::{bail, Result};
use clap::Parser;
use io_imap::rfc3501::rename::*;
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameArg, TargetMailboxNameArg},
};

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

        let mut arg = None;
        let mut coroutine = ImapMailboxRename::new(imap.context, from, to);

        loop {
            match coroutine.resume(arg.take()) {
                ImapMailboxRenameResult::Io { input } => {
                    arg = Some(handle(&mut imap.stream, input)?)
                }
                ImapMailboxRenameResult::Ok { .. } => break,
                ImapMailboxRenameResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Mailbox successfully renamed"))
    }
}
