//! # mbox flag set
//!
//! The `mbox flag set` command, replacing the whole flag set.

use anyhow::Result;
use clap::Parser;
use io_mbox::flag::MboxFlags;
use pimalaya_cli::printer::Printer;

use crate::{
    mbox::{
        arg::{MboxPathFlag, MessageIdsArg},
        client::MboxClient,
        flag::arg::FlagArg,
    },
    shared::receipt::IdsReceipt,
};

/// Set mbox flag(s) on message(s).
///
/// Replaces the flags of each message identified by the given id(s) with
/// the given ones, custom keywords included, in one rewrite of the file.
/// Every id is looked up first: an unknown one refuses the whole set,
/// nothing changed.
#[derive(Debug, Parser)]
pub struct MboxFlagSetCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub mbox: MboxPathFlag,
    /// Flag(s) of the message. Repeat `-f` per flag (e.g. `-f seen -f
    /// flagged`); a single `-f` takes one value so trailing message ids
    /// are not swallowed as flags.
    #[arg(long = "flag", short, value_name = "FLAG")]
    pub flags: Vec<FlagArg>,
}

impl MboxFlagSetCommand {
    /// Replaces the flag set of each message.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        let path = client.open_mbox(&self.mbox.inner)?;
        let flags = MboxFlags::from_iter(self.flags.into_iter().map(Into::into));
        let mut ids: Vec<&str> = Vec::with_capacity(self.ids.inner.len());
        for id in &self.ids.inner {
            if !ids.contains(&id.as_str()) {
                ids.push(id);
            }
        }

        client.edit_flags(&path, &ids, |_| flags.clone())?;

        printer.out(IdsReceipt {
            ids: ids.iter().map(ToString::to_string).collect(),
            done: "Flag(s) successfully changed",
        })
    }
}
