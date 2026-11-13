//! # mbox message move
//!
//! The `mbox message move` command, moving messages to the end of another mbox.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    mbox::{
        arg::{MboxPathFlag, MessageIdsArg, TargetMboxPathFlag},
        client::MboxClient,
    },
    shared::receipt::IdsReceipt,
};

/// Move mbox message(s) to another mbox.
///
/// Appends each message identified by the given id(s) to the target, then
/// removes it from the source. An interruption leaves the messages in both
/// files, never in neither.
///
/// Prints the ids the messages carry in the target, a message named
/// twice moved once. Every id is looked up first: an unknown one
/// refuses the whole set, nothing moved.
#[derive(Debug, Parser)]
pub struct MboxMessageMoveCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub source: MboxPathFlag,
    #[command(flatten)]
    pub target: TargetMboxPathFlag,
}

impl MboxMessageMoveCommand {
    /// Moves the messages to the target mbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();
        client.require_ids(&client.open_mbox(&self.source.inner)?, &ids)?;
        let ids = client.move_message_ids(&self.source.inner, &self.target.inner, &ids)?;
        printer.out(IdsReceipt {
            ids,
            done: "Message(s) successfully moved",
        })
    }
}
