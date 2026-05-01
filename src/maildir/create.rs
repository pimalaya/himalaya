use anyhow::{bail, Result};
use clap::Parser;
use io_maildir::coroutines::maildir_create::{
    MaildirCreate, MaildirCreateArg, MaildirCreateResult,
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::maildir::{account::MaildirAccount, arg::MaildirNameArg, runtime};

/// Create the given mailbox.
///
/// This command allows you to create a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct MaildirMailboxCreateCommand {
    #[command(flatten)]
    pub maildir_name: MaildirNameArg,
}

impl MaildirMailboxCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let path = account.backend.root.join(self.maildir_name.inner);

        let mut coroutine = MaildirCreate::new(path);
        let mut arg = None;

        loop {
            match coroutine.resume(arg.take()) {
                MaildirCreateResult::Ok => break,
                MaildirCreateResult::WantsDirCreate(paths) => {
                    runtime::dir_create(paths)?;
                    arg = Some(MaildirCreateArg::DirCreate);
                }
                MaildirCreateResult::Err(err) => bail!("{err}"),
            }
        }

        printer.out(Message::new("Maildir successfully created"))
    }
}
