//! # Gmail message batch delete
//!
//! The `gmail messages batch-delete` command,
//! `users.messages.batchDelete`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::messages::batch_delete::GmailMessagesBatchDelete;
use pimalaya_cli::printer::Printer;

use crate::{gmail::client::GmailClient, shared::receipt::IdsReceipt};

/// Permanently delete several Gmail messages at once
/// (users.messages.batchDelete).
#[derive(Debug, Parser)]
pub struct GmailMessageBatchDeleteCommand {
    /// The ids of the messages to delete.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl GmailMessageBatchDeleteCommand {
    /// Deletes every named message for good.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailMessagesBatchDelete::new(&client.auth, &client.user_id, &self.ids)?;
            client.run(c)?
        };

        printer.out(IdsReceipt {
            ids: self.ids,
            done: "Gmail messages permanently deleted",
        })
    }
}
