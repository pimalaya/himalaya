//! # Gmail label delete
//!
//! The `gmail labels delete` command, `users.labels.delete`.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{gmail::client::GmailClient, shared::receipt::IdReceipt};

/// Delete a Gmail label (users.labels.delete).
#[derive(Debug, Parser)]
pub struct GmailLabelDeleteCommand {
    /// Identifier of the label to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailLabelDeleteCommand {
    /// Deletes the label.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        client.label_delete(&self.id)?;

        printer.out(IdReceipt {
            id: self.id,
            done: "Gmail label successfully deleted",
        })
    }
}
