use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use pimalaya_cli::{printer::Printer, prompt};
use schemars::JsonSchema;
use serde::Serialize;

use crate::pimdir::client::PimdirClient;

/// Cancel one staged message, by the row id `queue list` prints.
///
/// This is the only way back for a queued creation: a staged flag or move is
/// undone by doing the opposite, but a message that does not exist yet cannot
/// be deleted. Cancelling is the store owner's write (pimdir SPEC §15.5), so
/// this takes that role for the length of the call and fails while a sync holds
/// it.
#[derive(Debug, Parser)]
pub struct PimdirQueueCancelCommand {
    /// Row id of the staged message, as `pimdir queue list` prints it.
    #[arg(value_name = "ROW")]
    pub id: i64,

    /// Do not ask for confirmation.
    #[arg(long, short)]
    pub yes: bool,
}

impl PimdirQueueCancelCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut PimdirClient) -> Result<()> {
        if !self.yes
            && !prompt::bool(
                format!("Cancel the message queued as row {}?", self.id),
                false,
            )?
        {
            bail!("Cancellation aborted");
        }

        if !client.cancel_queued(self.id)? {
            bail!(
                "No queued action with row {}; it may have been synced already",
                self.id
            );
        }

        printer.out(PimdirQueueCancelled { id: self.id })
    }
}

/// The `pimdir queue cancel` output.
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
pub struct PimdirQueueCancelled {
    /// The row that was cancelled.
    pub id: i64,
}

impl fmt::Display for PimdirQueueCancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Queued message {} cancelled", self.id)
    }
}
