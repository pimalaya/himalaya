//! # mbox rename
//!
//! The `mbox rename` command, renaming an mbox file under the account
//! root.

use anyhow::Result;
use clap::Parser;
use io_mbox::mbox::rename::MboxRename;
use pimalaya_cli::printer::Printer;

use crate::{
    mbox::{
        arg::{MboxNameArg, RequiredMboxPathFlag},
        client::{MboxClient, validate_mbox_name},
    },
    shared::receipt::IdReceipt,
};

/// Rename an mbox.
///
/// The source is named explicitly, with no default, renaming being
/// destructive. The target must not exist.
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
        validate_mbox_name(&self.mbox_path.inner)?;
        validate_mbox_name(&self.mbox_name.inner)?;
        let from = client.resolve_mbox(&self.mbox_path.inner);
        let to = client.resolve_mbox(&self.mbox_name.inner);
        client.run(MboxRename::new(from, to))?;
        printer.out(IdReceipt {
            id: self.mbox_name.inner,
            done: "Mbox successfully renamed",
        })
    }
}
