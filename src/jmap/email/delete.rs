//! # JMAP email delete
//!
//! The `jmap email delete` command, the destroy half of RFC 8621
//! `Email/set`.

use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::email::set::JmapEmailSetArgs;
use pimalaya_cli::printer::Printer;

use crate::{
    error::ErrorCode,
    jmap::{client::JmapClient, set::JmapSetReceipt},
};

/// Delete JMAP emails (Email/set destroy).
#[derive(Debug, Parser)]
pub struct JmapEmailDestroyCommand {
    /// Email ID(s) to delete.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapEmailDestroyCommand {
    /// Destroys the emails and reports any the server refused.
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let mut args = JmapEmailSetArgs::default();

        for id in self.ids {
            args.destroy(id);
        }

        let output = client.email_set(args)?;

        JmapSetReceipt::new(
            "JMAP emails successfully deleted",
            output.destroyed,
            output.not_destroyed,
            Some(ErrorCode::MessageNotFound),
        )
        .print(printer)
    }
}
