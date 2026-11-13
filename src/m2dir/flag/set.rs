//! # m2dir flag set
//!
//! The `m2dir flag set` command, replacing the whole flag set.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    m2dir::{
        arg::{M2dirNameFlag, MessageIdsArg},
        client::M2dirClient,
        flag::parse_flags,
    },
    shared::receipt::IdsReceipt,
};

/// Set m2dir flags on messages, replacing the ones they carry.
#[derive(Debug, Parser)]
pub struct M2dirFlagSetCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub m2dir: M2dirNameFlag,
    /// Flag(s) to set on the message, in the m2dir spelling. Repeat
    /// `-f` per flag (e.g. `-f '$seen' -f '$flagged'`); a single `-f`
    /// takes one value so trailing message ids are not swallowed as flags.
    #[arg(long = "flag", short = 'f', value_name = "FLAG")]
    pub flags: Vec<String>,
}

impl M2dirFlagSetCommand {
    /// Replaces the flag set of each message.
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;
        let flags = parse_flags(&self.flags)?;
        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();

        for entry in client.entries(&m2dir, &ids)? {
            client.set_flags(&m2dir, entry.id(), flags.clone())?;
        }

        printer.out(IdsReceipt {
            ids: self.ids.inner,
            done: "M2dir flag(s) successfully changed",
        })
    }
}
