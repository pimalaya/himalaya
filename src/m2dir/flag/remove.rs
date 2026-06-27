use anyhow::Result;
use clap::Parser;
use io_m2dir::flag::types::M2dirFlags;
use pimalaya_cli::printer::{Message, Printer};

use crate::m2dir::{
    arg::{M2dirNameFlag, MessageIdsArg},
    client::M2dirClient,
};

/// Remove M2DIR flag(s) from message(s).
#[derive(Debug, Parser)]
pub struct M2dirFlagRemoveCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,

    #[command(flatten)]
    pub m2dir: M2dirNameFlag,

    /// Flag(s) to remove from the message.
    #[arg(long = "flag", short = 'f', num_args = 1.., required = true)]
    pub flags: Vec<String>,
}

impl M2dirFlagRemoveCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;
        let flags = M2dirFlags::from_iter(self.flags.iter().map(String::as_str));

        for id in self.ids.inner {
            client.remove_flags(&m2dir, &id, flags.clone())?;
        }

        printer.out(Message::new("M2dir flag(s) successfully removed"))
    }
}
