//! # mbox create
//!
//! The `mbox create` command, creating an empty mbox file under the
//! account root.

use anyhow::Result;
use clap::Parser;
use io_mbox::mbox::create::MboxCreate;
use pimalaya_cli::printer::{Message, Printer};

use crate::mbox::{
    arg::MboxNameArg,
    client::{MboxClient, validate_mbox_name},
};

/// Create an mbox.
///
/// An empty file is created under the account root, with its parent
/// directories.
#[derive(Debug, Parser)]
pub struct MboxMailboxCreateCommand {
    #[command(flatten)]
    pub mbox_name: MboxNameArg,
}

impl MboxMailboxCreateCommand {
    /// Creates the mbox under the account root.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        validate_mbox_name(&self.mbox_name.inner)?;
        let path = client.resolve_mbox(&self.mbox_name.inner);
        client.run(MboxCreate::new(path))?;
        printer.out(Message::new("Mbox successfully created"))
    }
}
