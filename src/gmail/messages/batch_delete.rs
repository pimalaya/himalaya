use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::messages::batch_delete::GmailMessagesBatchDelete;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Permanently delete several Gmail messages at once
/// (users.messages.batchDelete).
#[derive(Debug, Parser)]
pub struct GmailMessageBatchDeleteCommand {
    /// The ids of the messages to delete.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl GmailMessageBatchDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let count = self.ids.len();

        {
            let c = GmailMessagesBatchDelete::new(&client.auth, &client.user_id, &self.ids)?;
            client.run(c)?
        };

        printer.out(Message::new(format!(
            "{count} messages permanently deleted"
        )))
    }
}
