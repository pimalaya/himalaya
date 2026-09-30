//! # mbox delete
//!
//! The `mbox delete` command, removing an mbox file and its messages.

use anyhow::Result;
use clap::Parser;
use io_mbox::mbox::delete::MboxDelete;
use pimalaya_cli::printer::{Message, Printer};

use crate::mbox::{
    arg::RequiredMboxPathFlag,
    client::{MboxClient, validate_mbox_name},
};

/// Delete an mbox.
///
/// The file and every message in it go, under lock so no delivery lands
/// in it meanwhile. The target is named explicitly, with no default,
/// deletion being destructive.
#[derive(Debug, Parser)]
pub struct MboxMailboxDeleteCommand {
    #[command(flatten)]
    pub mbox_path: RequiredMboxPathFlag,
}

impl MboxMailboxDeleteCommand {
    /// Deletes the mbox and every message in it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        validate_mbox_name(&self.mbox_path.inner)?;
        let path = client.resolve_mbox(&self.mbox_path.inner);
        client.run(MboxDelete::new(path, client.lock.clone()))?;
        printer.out(Message::new("Mbox successfully deleted"))
    }
}
