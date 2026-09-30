//! # mbox message move
//!
//! The `mbox message move` command, moving messages to the end of another mbox.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::mbox::{
    arg::{MboxPathFlag, MessageIdsArg, TargetMboxPathFlag},
    client::MboxClient,
};

/// Move mbox message(s) to another mbox.
///
/// Appends each message identified by the given id(s) to the target, then removes it from the source. An interruption leaves the messages in both files, never in neither.
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
        client.move_messages(&self.source.inner, &self.target.inner, &ids)?;
        printer.out(Message::new("Message(s) successfully moved"))
    }
}
