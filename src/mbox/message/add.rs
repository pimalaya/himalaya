//! # mbox message add
//!
//! The `mbox message add` command, appending a raw message to an mbox.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_mbox::flag::MboxFlags;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    mbox::{arg::MboxPathFlag, client::MboxClient, flag::arg::FlagArg},
    unified::message::arg::MessageArg,
};

/// Append a message to an mbox.
///
/// The message comes from a file path, an inline string or piped standard
/// input, and lands at the end of the file under lock, quoted in the
/// configured format.
#[derive(Debug, Parser)]
pub struct MboxMessageAddCommand {
    #[command(flatten)]
    pub mbox: MboxPathFlag,
    /// The flags to store with the message.
    #[arg(long = "flag", short)]
    pub flags: Vec<FlagArg>,
    #[command(flatten)]
    pub message: MessageArg,
}

impl MboxMessageAddCommand {
    /// Appends the raw message to the mbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        let path = client.open_mbox(&self.mbox.inner)?;
        let msg = self.message.parse()?;
        let flags = MboxFlags::from_iter(self.flags.into_iter().map(Into::into));

        let id = client.append(&path, flags, msg)?;

        printer.out(MboxMessageAddOutput { id })
    }
}

/// Output of an mbox message added: its id.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MboxMessageAddOutput {
    /// The id the mbox gave the message.
    id: String,
}

impl fmt::Display for MboxMessageAddOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Message `{}` successfully added", self.id)
    }
}
