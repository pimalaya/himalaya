use anyhow::{bail, Result};
use clap::Parser;
use io_maildir::coroutines::maildir_delete::{
    MaildirDelete, MaildirDeleteArg, MaildirDeleteResult,
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::maildir::{account::MaildirAccount, arg::MaildirPathFlag, runtime};

/// Delete the given mailbox.
///
/// This command allows you to delete a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct MaildirMailboxDeleteCommand {
    #[command(flatten)]
    pub maildir_path: MaildirPathFlag,
}

impl MaildirMailboxDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let path = account.backend.root.join(self.maildir_path.inner);

        let mut coroutine = MaildirDelete::new(path);
        let mut arg = None;

        loop {
            match coroutine.resume(arg.take()) {
                MaildirDeleteResult::Ok => break,
                MaildirDeleteResult::WantsDirRemove(paths) => {
                    runtime::dir_remove(paths)?;
                    arg = Some(MaildirDeleteArg::DirRemove);
                }
                MaildirDeleteResult::Err(err) => bail!("{err}"),
            }
        }

        printer.out(Message::new("Maildir successfully deleted"))
    }
}
