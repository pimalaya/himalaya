//! # m2dir flag add
//!
//! The `m2dir flag add` command, adding flags to the existing set.

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

/// Add m2dir flags to messages.
///
/// Each flag is an arbitrary UTF-8 string (e.g. `$seen`, `custom`).
#[derive(Debug, Parser)]
pub struct M2dirFlagAddCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub m2dir: M2dirNameFlag,
    /// Flag(s) to add to the message, in the m2dir spelling. Repeat
    /// `-f` per flag (e.g. `-f '$seen' -f '$flagged'`); a single `-f`
    /// takes one value so trailing message ids are not swallowed as flags.
    #[arg(long = "flag", short = 'f', value_name = "FLAG", required = true)]
    pub flags: Vec<String>,
}

impl M2dirFlagAddCommand {
    /// Adds the flags to the existing set of each message.
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;
        let flags = parse_flags(&self.flags)?;
        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();

        for entry in client.entries(&m2dir, &ids)? {
            client.add_flags(&m2dir, entry.id(), flags.clone())?;
        }

        printer.out(IdsReceipt {
            ids: self.ids.inner,
            done: "M2dir flag(s) successfully added",
        })
    }
}
