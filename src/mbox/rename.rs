//! # mbox rename
//!
//! The `mbox rename` command, renaming an mbox file under the account
//! root.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    mbox::{
        arg::{MboxNameArg, RequiredMboxPathFlag},
        client::MboxClient,
    },
    shared::receipt::IdReceipt,
};

/// Rename an mbox.
///
/// The source is named explicitly, with no default, renaming being
/// destructive. The target must not exist. In the Thunderbird layout
/// the `.sbd` directory of its children follows it.
#[derive(Debug, Parser)]
pub struct MboxMailboxRenameCommand {
    #[command(flatten)]
    pub mbox_path: RequiredMboxPathFlag,
    #[command(flatten)]
    pub mbox_name: MboxNameArg,
}

impl MboxMailboxRenameCommand {
    /// Renames the mbox under the account root.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        client.rename_mbox(&self.mbox_path.inner, &self.mbox_name.inner)?;
        printer.out(IdReceipt {
            id: self.mbox_name.inner,
            done: "Mbox successfully renamed",
        })
    }
}
