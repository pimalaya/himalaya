use anyhow::{bail, Result};
use clap::Parser;
use io_fs::runtimes::std::handle;
use io_maildir::coroutines::rename_maildir::*;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirNameArg, MaildirPathFlag},
};

/// Rename the given mailbox.
///
/// This command allows you to rename a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct MaildirMailboxRenameCommand {
    #[command(flatten)]
    pub maildir_path: MaildirPathFlag,
    #[command(flatten)]
    pub maildir_name: MaildirNameArg,
}

impl MaildirMailboxRenameCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let path = account.backend.root.join(self.maildir_path.inner);

        let mut arg = None;
        let mut coroutine = RenameMaildir::new(path, self.maildir_name.inner);

        loop {
            match coroutine.resume(arg.take()) {
                RenameMaildirResult::Ok => break,
                RenameMaildirResult::Io(io) => arg = Some(handle(io)?),
                RenameMaildirResult::Err(err) => bail!(err),
            }
        }

        printer.out(Message::new("Maildir successfully renamed"))
    }
}
