use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::labels::GmailLabel;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Create a Gmail label (users.labels.create).
#[derive(Debug, Parser)]
pub struct GmailLabelCreateCommand {
    /// Display name of the label to create.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl GmailLabelCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let label = GmailLabel {
            name: self.name.clone(),
            ..Default::default()
        };
        let label = client.label_create(&label)?.response;

        printer.out(Message::new(format!(
            "Gmail label `{}` successfully created",
            label.id
        )))
    }
}
