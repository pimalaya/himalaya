use anyhow::{bail, Result};
use clap::Parser;
use io_fs::runtimes::std::handle;
use io_maildir::coroutines::delete_maildir::*;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::maildir::{account::MaildirAccount, arg::MaildirPathFlag};

/// Delete the given mailbox.
///
/// This command allows you to delete a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct DeleteMaildirCommand {
    #[command(flatten)]
    pub maildir_path: MaildirPathFlag,
}

impl DeleteMaildirCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let path = account.backend.root.join(self.maildir_path.inner);

        let mut arg = None;
        let mut coroutine = DeleteMaildir::new(path);

        loop {
            match coroutine.resume(arg.take()) {
                DeleteMaildirResult::Ok => break,
                DeleteMaildirResult::Io(io) => arg = Some(handle(io)?),
                DeleteMaildirResult::Err(err) => bail!(err),
            }
        }

        printer.out(Message::new("Maildir successfully deleted"))
    }
}
