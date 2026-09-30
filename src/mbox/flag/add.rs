//! # mbox flag add
//!
//! The `mbox flag add` command, adding flags to the existing set.

use anyhow::Result;
use clap::Parser;
use io_mbox::flag::MboxFlags;
use pimalaya_cli::printer::{Message, Printer};

use crate::mbox::{
    arg::{MboxPathFlag, MessageIdsArg},
    client::MboxClient,
    flag::arg::FlagArg,
};

/// Add mbox flag(s) to message(s).
///
/// Adds the given flags to the status headers of each message identified by the given id(s), in one rewrite of the file.
#[derive(Debug, Parser)]
pub struct MboxFlagAddCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub mbox: MboxPathFlag,
    /// Flag(s) of the message. Repeat `-f` per flag (e.g. `-f seen -f
    /// flagged`); a single `-f` takes one value so trailing message ids
    /// are not swallowed as flags.
    #[arg(long = "flag", short, value_name = "FLAG", required = true)]
    pub flags: Vec<FlagArg>,
}

impl MboxFlagAddCommand {
    /// Adds the flags to the existing set of each message.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        let path = client.resolve_mbox(&self.mbox.inner);
        let flags = MboxFlags::from_iter(self.flags.into_iter().map(Into::into));
        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();

        client.edit_flags(&path, &ids, |current| {
            current
                .iter()
                .cloned()
                .chain(flags.iter().cloned())
                .collect()
        })?;

        printer.out(Message::new("Flag(s) successfully added"))
    }
}
