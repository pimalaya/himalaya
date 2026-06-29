use anyhow::Result;
use clap::Parser;
use io_maildir::flag::types::MaildirFlags;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::{MaildirPathFlag, MessageIdsArg},
    client::MaildirClient,
    flag::arg::FlagArg,
};

/// Set MAILDIR flag(s) on message(s).
///
/// Replaces the info flags in the filename of each message identified
/// by the given id(s) with the given flags.
#[derive(Debug, Parser)]
pub struct MaildirFlagSetCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,

    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    /// Flag(s) to set on the message
    #[arg(long = "flag", short, num_args = 1..)]
    pub flags: Vec<FlagArg>,
}

impl MaildirFlagSetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let maildir = client.resolve_maildir(&self.maildir.inner)?;

        let flags = MaildirFlags::from_iter(self.flags.into_iter().map(Into::into));

        for id in self.ids.inner {
            client.set_flags(maildir.clone(), id, flags.clone())?;
        }

        printer.out(Message::new("Flag(s) successfully changed"))
    }
}
