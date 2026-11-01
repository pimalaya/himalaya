//! # pimdir queue cancel
//!
//! The `pimdir queue cancel` command, retracting one queued or parked
//! action before its owner applies it.

use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use io_pimdir::client::producer::PimdirActionStatus;
use pimalaya_cli::{printer::Printer, prompt, table::sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::pimdir::client::PimdirClient;

/// Cancel one queued or parked action, by the row id `queue list` prints.
///
/// This is the only way back for a queued creation: a staged flag or move is
/// undone by doing the opposite, where a message that does not exist yet
/// cannot be deleted. Only a row of the account's queue is taken.
///
/// Cancelling is the store owner's write, so this takes that role for the
/// length of the call and fails while a sync holds it. A store another
/// version of the sync engine wrote is refused rather than reshaped.
#[derive(Debug, Parser)]
pub struct PimdirQueueCancelCommand {
    /// Row id of the queued action, as `pimdir queue list` prints it.
    #[arg(value_name = "ROW")]
    pub id: i64,
    /// Do not ask for confirmation.
    #[arg(long, short)]
    pub yes: bool,
}

impl PimdirQueueCancelCommand {
    /// Retracts the queued action the row id names.
    pub fn execute(self, printer: &mut impl Printer, client: &mut PimdirClient) -> Result<()> {
        let named = match client.queue_row(self.id)? {
            PimdirActionStatus::Pending { kind, .. } | PimdirActionStatus::Parked { kind, .. } => {
                kind
            }
            _ => String::from("action"),
        };
        if !self.yes
            && !prompt::bool(
                format!("Cancel the {} queued as row {}?", sanitize(&named), self.id),
                false,
            )?
        {
            bail!("Cancellation aborted");
        }

        let kind = client.cancel_queued(self.id)?;
        printer.out(PimdirQueueCancelOutput { id: self.id, kind })
    }
}

/// The `pimdir queue cancel` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueueCancelOutput {
    /// The row that was cancelled.
    pub id: i64,
    /// The action kind it held: `add`, `submit`, `set-flags`, ...
    pub kind: String,
}

impl fmt::Display for PimdirQueueCancelOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Queued {} cancelled (row {})",
            sanitize(&self.kind),
            self.id
        )
    }
}
