//! # Gmail label delete
//!
//! The `gmail labels delete` command, `users.labels.delete`.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

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

        printer.out(Message::new(format!(
            "Gmail label `{}` successfully deleted",
            self.id
        )))
    }
}
