//! # JMAP identity delete
//!
//! The `jmap identity delete` command, the destroy half of RFC 8621
//! `Identity/set`.

use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::identity::set::JmapIdentitySetArgs;
use pimalaya_cli::printer::Printer;

use crate::{
    error::ErrorCode,
    jmap::{client::JmapClient, set::JmapSetReceipt},
};

/// Delete a JMAP sender identity (Identity/set).
#[derive(Debug, Parser)]
pub struct JmapIdentityDeleteCommand {
    /// Identity ID(s) to delete.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapIdentityDeleteCommand {
    /// Destroys the identity.
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let mut args = JmapIdentitySetArgs::default();

        for id in self.ids {
            args.destroy(id);
        }

        let output = client.identity_set(args)?;

        JmapSetReceipt::new(
            "JMAP identities successfully deleted",
            output.destroyed,
            output.not_destroyed,
            Some(ErrorCode::IdentityNotFound),
        )
        .print(printer)
    }
}
