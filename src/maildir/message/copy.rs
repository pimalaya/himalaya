use anyhow::{bail, Result};
use clap::Parser;
use io_maildir::{
    coroutines::message_copy::{
        MaildirMessageCopy, MaildirMessageCopyArg, MaildirMessageCopyResult,
    },
    maildir::Maildir,
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirPathFlag, MaildirSubdirArg, MessageIdsArg, TargetMaildirPathFlag},
    runtime,
};

/// Copy Maildir message to the given mailbox.
///
/// This command copies message(s) identified by the given sequence
/// set from the source mailbox to the destination mailbox.
#[derive(Debug, Parser)]
pub struct MaildirMessageCopyCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub source: MaildirPathFlag,
    #[command(flatten)]
    pub target: TargetMaildirPathFlag,

    /// Copy the message into a different subdirectory.
    #[arg(long, short, value_name = "DIR", value_enum)]
    pub subdir: Option<MaildirSubdirArg>,
}

impl MaildirMessageCopyCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let source = match Maildir::try_from(self.source.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.source.inner))?,
        };

        let target = match Maildir::try_from(self.target.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.target.inner))?,
        };

        for id in self.ids.inner {
            let mut coroutine = MaildirMessageCopy::new(
                id,
                source.clone(),
                target.clone(),
                self.subdir.clone().map(Into::into),
            );
            let mut arg = None;

            loop {
                match coroutine.resume(arg.take()) {
                    MaildirMessageCopyResult::Ok => break,
                    MaildirMessageCopyResult::WantsDirRead(paths) => {
                        arg = Some(MaildirMessageCopyArg::DirRead(runtime::dir_read(paths)?));
                    }
                    MaildirMessageCopyResult::WantsCopy(pairs) => {
                        runtime::copy(pairs)?;
                        arg = Some(MaildirMessageCopyArg::Copy);
                    }
                    MaildirMessageCopyResult::Err(err) => bail!("{err}"),
                }
            }
        }

        printer.out(Message::new("Message(s) successfully copied"))
    }
}
