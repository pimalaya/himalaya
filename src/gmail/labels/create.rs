//! # Gmail label create
//!
//! The `gmail labels create` command, `users.labels.create`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::labels::GmailLabel;
use pimalaya_cli::printer::Printer;

use crate::{gmail::client::GmailClient, shared::receipt::IdReceipt};

/// Create a Gmail label (users.labels.create).
#[derive(Debug, Parser)]
pub struct GmailLabelCreateCommand {
    /// Display name of the label to create.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl GmailLabelCreateCommand {
    /// Creates the label and reports its new id.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let label = GmailLabel {
            name: self.name.clone(),
            ..Default::default()
        };
        let label = client.label_create(&label)?.response;

        printer.out(IdReceipt {
            id: label.id,
            done: "Gmail label successfully created",
        })
    }
}
