use anyhow::Result;
use clap::Parser;
use io_maildir::{flag::Flags, maildir::Maildir};
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    account::MaildirAccount,
    arg::{MaildirPathFlag, MessageIdsArg},
    flag::arg::FlagArg,
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
            Err(_) => Maildir::try_from(account.backend.root.join(&self.maildir.inner))?,
        };

        let flags = Flags::from_iter(self.flags.into_iter().map(Into::into));
        let client = account.new_maildir_client();

        for id in self.ids.inner {
            client.add_flags(maildir.clone(), id, flags.clone())?;
        }

        printer.out(Message::new("Flag(s) successfully added"))
    }
}
