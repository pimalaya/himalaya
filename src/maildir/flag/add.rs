use anyhow::Result;
use clap::Parser;
use io_maildir::flag::types::MaildirFlags;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::{MaildirPathFlag, MessageIdsArg},
    client::MaildirClient,
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
    /// MaildirFlag(s) to add to the message
    #[arg(long = "flag", short, num_args = 1..)]
    pub flags: Vec<FlagArg>,
}

impl MaildirFlagAddCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let maildir = client.resolve_maildir(&self.maildir.inner)?;

        let flags = MaildirFlags::from_iter(self.flags.into_iter().map(Into::into));

        for id in self.ids.inner {
            client.add_flags(maildir.clone(), id, flags.clone())?;
        }

        printer.out(Message::new("MaildirFlag(s) successfully added"))
    }
}
