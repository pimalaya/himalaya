use anyhow::Result;
use clap::Parser;
use io_maildir::flag::types::MaildirFlags;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::{MaildirPathFlag, MessageIdsArg},
    client::MaildirClient,
    flag::arg::FlagArg,
};

/// Remove MAILDIR flag(s) from message(s).
///
/// Removes the given info flags from the filename of each message
/// identified by the given id(s).
#[derive(Debug, Parser)]
pub struct MaildirFlagRemoveCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,

    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    /// Flag(s) to remove from the message
    #[arg(long = "flag", short, num_args = 1.., required = true)]
    pub flags: Vec<FlagArg>,
}

impl MaildirFlagRemoveCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let maildir = client.resolve_maildir(&self.maildir.inner)?;

        let flags = MaildirFlags::from_iter(self.flags.into_iter().map(Into::into));

        for id in self.ids.inner {
            client.remove_flags(maildir.clone(), id, flags.clone())?;
        }

        printer.out(Message::new("Flag(s) successfully removed"))
    }
}
