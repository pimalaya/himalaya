use anyhow::Result;
use clap::Parser;
use io_maildir::{flag::Flags, maildir::Maildir};
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::{MaildirPathFlag, MessageIdsArg},
    client::MaildirClient,
    flag::arg::FlagArg,
};

/// Remove MAILDIR flag(s) to message(s).
///
/// This command removes the given flags to messages identified by the
/// given sequence set.
#[derive(Debug, Parser)]
pub struct MaildirFlagRemoveCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,

    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    /// Flag(s) to remove to the message
    #[arg(long = "flag", short, num_args = 1..)]
    pub flags: Vec<FlagArg>,
}

impl MaildirFlagRemoveCommand {
    pub fn execute(self, printer: &mut impl Printer, client: MaildirClient) -> Result<()> {
        let maildir = match Maildir::try_from(self.maildir.inner.clone()) {
            Ok(maildir) => maildir,
            Err(_) => Maildir::try_from(client.root.join(&self.maildir.inner))?,
        };

        let flags = Flags::from_iter(self.flags.into_iter().map(Into::into));

        for id in self.ids.inner {
            client.remove_flags(maildir.clone(), id, flags.clone())?;
        }

        printer.out(Message::new("Flag(s) successfully removed"))
    }
}
