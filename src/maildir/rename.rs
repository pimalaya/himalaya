use anyhow::{bail, Result};
use clap::Parser;
use io_maildir::coroutines::maildir_rename::{
    MaildirRename, MaildirRenameArg, MaildirRenameResult,
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirNameArg, MaildirPathFlag},
    runtime,
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

        let mut coroutine = MaildirRename::new(path, self.maildir_name.inner);
        let mut arg = None;

        loop {
            match coroutine.resume(arg.take()) {
                MaildirRenameResult::Ok => break,
                MaildirRenameResult::WantsRename(pairs) => {
                    runtime::rename(pairs)?;
                    arg = Some(MaildirRenameArg::Rename);
                }
                MaildirRenameResult::Err(err) => bail!("{err}"),
            }
        }

        printer.out(Message::new("Maildir successfully renamed"))
    }
}
