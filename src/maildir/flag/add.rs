//! # Maildir flag add
//!
//! The `maildir flag add` command, adding flags to the existing set.

use anyhow::Result;
use clap::Parser;
use io_maildir::flag::MaildirFlags;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::{MaildirPathFlag, MessageIdsArg},
    client::MaildirClient,
    flag::arg::FlagArg,
};

/// Add MAILDIR flag(s) to message(s).
///
/// Appends the given info flags to the filename of each message
/// identified by the given id(s).
#[derive(Debug, Parser)]
pub struct MaildirFlagAddCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub maildir: MaildirPathFlag,
    /// Flag(s) to add to the message. Repeat `-f` per flag (e.g. `-f
    /// seen -f flagged`); a single `-f` takes one value so trailing
    /// message ids are not swallowed as flags.
    #[arg(long = "flag", short, value_name = "FLAG", required = true)]
    pub flags: Vec<FlagArg>,
}

impl MaildirFlagAddCommand {
    /// Adds the flags to the existing set of each message.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        let maildir = client.resolve_maildir(&self.maildir.inner)?;

        let flags = MaildirFlags::from_iter(self.flags.into_iter().map(Into::into));

        for id in self.ids.inner {
            client.add_flags(maildir.clone(), id, flags.clone())?;
        }

        printer.out(Message::new("Flag(s) successfully added"))
    }
}
