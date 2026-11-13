//! # mbox create
//!
//! The `mbox create` command, creating an empty mbox file under the
//! account root.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    mbox::{arg::MboxNameArg, client::MboxClient},
    shared::receipt::IdReceipt,
};

/// Create an mbox.
///
/// An empty file is created under the account root, with its parent
/// directories. In the plain layout an mbox file holds no mailbox, so a
/// name under one is refused; `mbox.thunderbird` nests them.
#[derive(Debug, Parser)]
pub struct MboxMailboxCreateCommand {
    #[command(flatten)]
    pub mbox_name: MboxNameArg,
}

impl MboxMailboxCreateCommand {
    /// Creates the mbox under the account root.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        client.create_mailbox(&self.mbox_name.inner, None)?;
        printer.out(IdReceipt {
            id: self.mbox_name.inner,
            done: "Mbox successfully created",
        })
    }
}
