use anyhow::{bail, Result};
use clap::Parser;
use io_fs::runtimes::std::handle;
use io_maildir::coroutines::create_maildir::*;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::maildir::{account::MaildirAccount, arg::MaildirNameArg};

/// Create the given mailbox.
///
/// This command allows you to create a new mailbox using the given
/// name.
#[derive(Debug, Parser)]
pub struct CreateMaildirCommand {
    #[command(flatten)]
    pub maildir_name: MaildirNameArg,
}

impl CreateMaildirCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let path = account.backend.root.join(self.maildir_name.inner);

        let mut arg = None;
        let mut coroutine = CreateMaildir::new(path);

        loop {
            match coroutine.resume(arg.take()) {
                CreateMaildirResult::Ok => break,
                CreateMaildirResult::Io(io) => arg = Some(handle(io)?),
                CreateMaildirResult::Err(err) => bail!(err),
            }
        }

        printer.out(Message::new("Maildir successfully created"))
    }
}
