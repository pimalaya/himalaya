//! # mbox message save
//!
//! The `mbox message save` command, appending a raw message to an mbox.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_mbox::flag::MboxFlags;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    mbox::{arg::MboxPathFlag, client::MboxClient, flag::arg::FlagArg},
    shared::message::arg::MessageArg,
};

/// Append a message to an mbox.
///
/// The message comes from a file path, an inline string or piped standard
/// input, and lands at the end of the file under lock, quoted in the
/// configured format.
#[derive(Debug, Parser)]
pub struct MboxMessageSaveCommand {
    #[command(flatten)]
    pub mbox: MboxPathFlag,
    /// The flags to store with the message.
    #[arg(long = "flag", short, num_args = 0..)]
    pub flags: Vec<FlagArg>,
    #[command(flatten)]
    pub message: MessageArg,
}

impl MboxMessageSaveCommand {
    /// Appends the raw message to the mbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        let path = client.resolve_mbox(&self.mbox.inner);
        let msg = self.message.parse()?;
        let flags = MboxFlags::from_iter(self.flags.into_iter().map(Into::into));

        let id = client.append(&path, flags, msg.into_bytes())?;

        printer.out(StoredMessage { id })
    }
}

/// Output of a saved mbox message: its id.
#[derive(Serialize, JsonSchema)]
pub struct StoredMessage {
    id: String,
}

impl fmt::Display for StoredMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Message `{}` successfully saved", self.id)
    }
}
