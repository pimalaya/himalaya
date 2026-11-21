//! # pimdir queue cancel
//!
//! The `pimdir queue cancel` command, retracting one queued or parked
//! action before its owner applies it.

use std::{
    fmt,
    io::{IsTerminal, stdin},
};

use anyhow::{Result, bail};
use clap::Parser;
use io_pimdir::{client::producer::PimdirActionStatus, codec::PimdirActionHandle};
use pimalaya_cli::{error::CodedError, printer::Printer, prompt, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    error::ErrorCode,
    pimdir::client::{PimdirClient, uncancellable},
};

/// Cancel one queued or parked action, by the handle a receipt prints or
/// the row id `queue list` prints.
///
/// This is the only way back for a queued creation: a staged flag or move is
/// undone by doing the opposite, where a message that does not exist yet
/// cannot be deleted. Only a row of the account's queue is taken.
///
/// An unconfirmed row, a send or a mailbox change the sync engine sent
/// without learning its outcome, is cancelled the same way, which undoes
/// nothing: the message may have been sent, and both the question and the
/// answer say so.
///
/// Cancelling is the store owner's write, so this takes that role for the
/// length of the call and fails while a sync holds it. A store another
/// version of the sync engine wrote is refused rather than reshaped.
#[derive(Debug, Parser)]
pub struct PimdirQueueCancelCommand {
    /// The handle of the queued action, as a receipt prints it, or its bare
    /// row id, as `pimdir queue list` prints it.
    #[arg(value_name = "ROW")]
    pub id: PimdirActionHandle,
    /// Do not ask for confirmation.
    #[arg(long, short)]
    pub yes: bool,
}

impl PimdirQueueCancelCommand {
    /// Retracts the queued action the row id names.
    pub fn execute(self, printer: &mut impl Printer, client: &mut PimdirClient) -> Result<()> {
        let (named, unconfirmed) = match client.queue_row(&self.id)? {
            PimdirActionStatus::Pending { kind, .. } | PimdirActionStatus::Parked { kind, .. } => {
                (kind, false)
            }
            PimdirActionStatus::Unconfirmed { kind, .. } => (kind, true),
            status => return Err(uncancellable(&self.id, &status)),
        };
        let question = match unconfirmed {
            true => format!(
                "The {} queued as row {} was sent and may have taken effect, which \
                 cancelling does not undo. Cancel it?",
                sanitize::text(&named),
                self.id
            ),
            false => format!(
                "Cancel the {} queued as row {}?",
                sanitize::text(&named),
                self.id
            ),
        };
        if !self.yes && !stdin().is_terminal() {
            let message = format!(
                "Cannot ask to cancel row {} with no terminal: pass --yes",
                self.id
            );
            return Err(CodedError::new(ErrorCode::ConfirmationRequired, message).into());
        }

        if !self.yes && !prompt::bool(question, false)? {
            bail!("Cancellation aborted");
        }

        let handle = client.named(&self.id)?;
        let kind = client.cancel_queued(&self.id)?;
        printer.out(PimdirQueueCancelOutput {
            row: handle.id,
            handle: handle.to_string(),
            kind,
            unconfirmed,
        })
    }
}

/// The `pimdir queue cancel` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirQueueCancelOutput {
    /// The queue row id cancelled.
    pub row: i64,
    /// The row's handle, its id under the store's identity (pimdir STORAGE
    /// §4.5), a bare id given named under this store.
    pub handle: String,
    /// The action kind it held: `add`, `submit`, `set-flags`, ...
    pub kind: String,
    /// Whether the row was unconfirmed, its request sent and its outcome
    /// unknown: the action may have taken effect all the same.
    pub unconfirmed: bool,
}

impl fmt::Display for PimdirQueueCancelOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = sanitize::text(&self.kind);
        let id = &self.handle;
        match self.unconfirmed {
            true => writeln!(
                f,
                "Unconfirmed {kind} cancelled (row {id}); it was sent and may have taken effect"
            ),
            false => writeln!(f, "Queued {kind} cancelled (row {id})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cancelling an unconfirmed row undoes nothing already sent (pimdir
    /// STORAGE §15.2), so the output never reads as a plain cancellation.
    #[test]
    fn cancelling_an_unconfirmed_row_says_it_may_have_taken_effect() {
        let cancelled = PimdirQueueCancelOutput {
            row: 9,
            handle: "e2646df031cd876334410c4df14ebd86.1.9".into(),
            kind: "submit".into(),
            unconfirmed: true,
        };

        let json = serde_json::to_value(&cancelled).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "row": 9,
                "handle": "e2646df031cd876334410c4df14ebd86.1.9",
                "kind": "submit",
                "unconfirmed": true,
            })
        );
        let line = cancelled.to_string();
        assert!(line.contains("may have taken effect"), "{line}");
    }
}
