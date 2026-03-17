use anyhow::{bail, Result};
use clap::Parser;
use io_fs::runtimes::std::handle;
use io_maildir::{coroutines::remove_flags::*, flag::Flags, maildir::Maildir};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirPathFlag, MessageIdsArg},
    flag::arg::FlagArg,
};

/// Remove MAILDIR flag(s) to message(s).
///
/// This command removes the given flags to messages identified by the
/// given sequence set.
#[derive(Debug, Parser)]
pub struct RemoveFlagsCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,

    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    /// Flag(s) to remove to the message
    #[arg(long = "flag", short, num_args = 1..)]
    pub flags: Vec<FlagArg>,
}

impl RemoveFlagsCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(account.backend.root.join(self.maildir.inner))?,
        };

        let flags = Flags::from_iter(self.flags.into_iter().map(Into::into));

        for id in self.ids.inner {
            let mut arg = None;
            let mut coroutine = RemoveMaildirFlags::new(maildir.clone(), id, flags.clone());

            loop {
                match coroutine.resume(arg.take()) {
                    RemoveMaildirFlagsResult::Ok => break,
                    RemoveMaildirFlagsResult::Io(io) => arg = Some(handle(io)?),
                    RemoveMaildirFlagsResult::Err(err) => bail!(err),
                }
            }
        }

        printer.out(Message::new("Flag(s) successfully removed"))
    }
}
