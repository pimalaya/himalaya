use anyhow::{bail, Result};
use clap::Parser;
use io_maildir::{
    coroutines::flags_set::{MaildirFlagsSet, MaildirFlagsSetArg, MaildirFlagsSetResult},
    flag::Flags,
    maildir::Maildir,
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirPathFlag, MessageIdsArg},
    flag::arg::FlagArg,
    runtime,
};

/// Set MAILDIR flag(s) to message(s).
///
/// This command sets the given flags to messages identified by the
/// given sequence set.
#[derive(Debug, Parser)]
pub struct MaildirFlagSetCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,

    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    /// Flag(s) to set to the message
    #[arg(long = "flag", short, num_args = 1..)]
    pub flags: Vec<FlagArg>,
}

impl MaildirFlagSetCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.maildir.inner))?,
        };

        let flags = Flags::from_iter(self.flags.into_iter().map(Into::into));

        for id in self.ids.inner {
            let mut coroutine = MaildirFlagsSet::new(maildir.clone(), id, flags.clone());
            let mut arg = None;

            loop {
                match coroutine.resume(arg.take()) {
                    MaildirFlagsSetResult::Ok => break,
                    MaildirFlagsSetResult::WantsDirRead(paths) => {
                        arg = Some(MaildirFlagsSetArg::DirRead(runtime::dir_read(paths)?));
                    }
                    MaildirFlagsSetResult::WantsRename(pairs) => {
                        runtime::rename(pairs)?;
                        arg = Some(MaildirFlagsSetArg::Rename);
                    }
                    MaildirFlagsSetResult::Err(err) => bail!("{err}"),
                }
            }
        }

        printer.out(Message::new("Flag(s) successfully changed"))
    }
}
