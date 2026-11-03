//! # m2dir flag set
//!
//! The `m2dir flag set` command, replacing the whole flag set.

use anyhow::Result;
use clap::Parser;
use io_m2dir::flag::M2dirFlags;
use pimalaya_cli::printer::Printer;

use crate::{
    m2dir::{
        arg::{M2dirNameFlag, MessageIdsArg},
        client::M2dirClient,
    },
    shared::receipt::IdsReceipt,
};

/// Set M2DIR flag(s) on message(s) (replaces any existing flags).
#[derive(Debug, Parser)]
pub struct M2dirFlagSetCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub m2dir: M2dirNameFlag,
    /// Flag(s) to set on the message. Repeat `-f` per flag (e.g. `-f
    /// seen -f flagged`); a single `-f` takes one value so trailing
    /// message ids are not silently swallowed as flags.
    #[arg(long = "flag", short = 'f', value_name = "FLAG")]
    pub flags: Vec<String>,
}

impl M2dirFlagSetCommand {
    /// Replaces the flag set of each message.
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;
        let flags = M2dirFlags::from_iter(self.flags.iter().map(String::as_str));

        for id in self.ids.inner.clone() {
            client.set_flags(&m2dir, &id, flags.clone())?;
        }

        printer.out(IdsReceipt {
            ids: self.ids.inner,
            done: "M2dir flag(s) successfully changed",
        })
    }
}
