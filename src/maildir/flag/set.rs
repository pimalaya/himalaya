use anyhow::{bail, Result};
use clap::Parser;
use io_fs::runtimes::std::handle;
use io_maildir::{coroutines::set_flags::*, flag::Flags, maildir::Maildir};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirPathFlag, MessageIdsArg},
    flag::arg::FlagArg,
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
            let mut arg = None;
            let mut coroutine = SetMaildirFlags::new(maildir.clone(), id, flags.clone());

            loop {
                match coroutine.resume(arg.take()) {
                    SetMaildirFlagsResult::Ok => break,
                    SetMaildirFlagsResult::Io(io) => arg = Some(handle(io)?),
                    SetMaildirFlagsResult::Err(err) => bail!(err),
                }
            }
        }

        printer.out(Message::new("Flag(s) successfully changed"))
    }
}
