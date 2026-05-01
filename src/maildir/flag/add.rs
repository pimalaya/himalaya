use anyhow::{bail, Result};
use clap::Parser;
use io_maildir::{
    coroutines::flags_add::{MaildirFlagsAdd, MaildirFlagsAddArg, MaildirFlagsAddResult},
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

/// Add MAILDIR flag(s) to message(s).
///
/// This command adds the given flags to messages identified by the
/// given sequence set.
#[derive(Debug, Parser)]
pub struct MaildirFlagAddCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,

    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    /// Flag(s) to add to the message
    #[arg(long = "flag", short, num_args = 1..)]
    pub flags: Vec<FlagArg>,
}

impl MaildirFlagAddCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.maildir.inner))?,
        };

        let flags = Flags::from_iter(self.flags.into_iter().map(Into::into));

        for id in self.ids.inner {
            let mut coroutine = MaildirFlagsAdd::new(maildir.clone(), id, flags.clone());
            let mut arg = None;

            loop {
                match coroutine.resume(arg.take()) {
                    MaildirFlagsAddResult::Ok => break,
                    MaildirFlagsAddResult::WantsDirRead(paths) => {
                        arg = Some(MaildirFlagsAddArg::DirRead(runtime::dir_read(paths)?));
                    }
                    MaildirFlagsAddResult::WantsRename(pairs) => {
                        runtime::rename(pairs)?;
                        arg = Some(MaildirFlagsAddArg::Rename);
                    }
                    MaildirFlagsAddResult::Err(err) => bail!("{err}"),
                }
            }
        }

        printer.out(Message::new("Flag(s) successfully added"))
    }
}
