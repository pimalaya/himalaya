use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::labels::GmailLabel;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Update a Gmail label name (users.labels.update).
#[derive(Debug, Parser)]
pub struct GmailLabelUpdateCommand {
    /// Identifier of the label to update.
    #[arg(value_name = "ID")]
    pub id: String,
    /// New display name to set on the label.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl GmailLabelUpdateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let label = GmailLabel {
            id: self.id.clone(),
            name: self.name.clone(),
            ..Default::default()
        };
        client.label_update(&label)?;

        printer.out(Message::new(format!(
            "Gmail label `{}` successfully updated",
            self.id
        )))
    }
}
