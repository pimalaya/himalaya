//! # mbox delete
//!
//! The `mbox delete` command, removing an mbox file and its messages.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    mbox::{arg::RequiredMboxPathFlag, client::MboxClient},
    shared::receipt::IdReceipt,
};

/// Delete an mbox.
///
/// The file and every message in it go, under lock so no delivery lands
/// in it meanwhile. The target is named explicitly, with no default,
/// deletion being destructive. In the Thunderbird layout an mbox holding
/// others is refused.
#[derive(Debug, Parser)]
pub struct MboxMailboxDeleteCommand {
    #[command(flatten)]
    pub mbox_path: RequiredMboxPathFlag,
}

impl MboxMailboxDeleteCommand {
    /// Deletes the mbox and every message in it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        client.delete_mailbox(&self.mbox_path.inner)?;
        printer.out(IdReceipt {
            id: self.mbox_path.inner,
            done: "Mbox successfully deleted",
        })
    }
}
