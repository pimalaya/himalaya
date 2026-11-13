//! # mbox message copy
//!
//! The `mbox message copy` command, copying messages to the end of another
//! mbox.

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

/// Copy mbox message(s) to another mbox.
///
/// Appends a copy of each message identified by the given id(s) to the
/// target, with its flags and `From_` line.
///
/// Prints the ids the copies carry in the target, a message named twice
/// copied once. Every id is looked up first: an unknown one refuses the
/// whole set, nothing copied.
#[derive(Debug, Parser)]
pub struct MboxMessageCopyCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub source: MboxPathFlag,
    #[command(flatten)]
    pub target: TargetMboxPathFlag,
}

impl MboxMessageCopyCommand {
    /// Copies the messages to the target mbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();
        client.require_ids(&client.open_mbox(&self.source.inner)?, &ids)?;
        let ids = client.copy_message_ids(&self.source.inner, &self.target.inner, &ids)?;
        printer.out(IdsReceipt {
            ids,
            done: "Message(s) successfully copied",
        })
    }
}
