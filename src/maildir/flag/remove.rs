//! # Maildir flag remove
//!
//! The `maildir flag remove` command, removing flags from the existing
//! set.

use anyhow::Result;
use clap::Parser;
use io_maildir::flag::MaildirFlags;
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
    /// Flag(s) to remove from the message. Repeat `-f` per flag (e.g.
    /// `-f seen -f flagged`); a single `-f` takes one value so trailing
    /// message ids are not swallowed as flags.
    #[arg(long = "flag", short, value_name = "FLAG", required = true)]
    pub flags: Vec<FlagArg>,
}

impl MaildirFlagRemoveCommand {
    /// Removes the flags from the existing set of each message.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let maildir = client.resolve_maildir(&self.maildir.inner)?;

        let flags = MaildirFlags::from_iter(self.flags.into_iter().map(Into::into));

        for id in self.ids.inner {
            client.remove_flags(maildir.clone(), id, flags.clone())?;
        }

        printer.out(Message::new("Flag(s) successfully removed"))
    }
}
