use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, labels::list::LabelsTable},
};

/// Get one or more Gmail labels by identifier (users.labels.get).
#[derive(Debug, Parser)]
pub struct GmailLabelGetCommand {
    /// Identifiers of the labels to get.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl GmailLabelGetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let mut labels = Vec::with_capacity(self.ids.len());

        for id in self.ids {
            labels.push(client.label_get(&id)?.response);
        }

        printer.out(LabelsTable::new(account, labels))
    }
}
