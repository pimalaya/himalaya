//! # Gmail message batch modify
//!
//! The `gmail messages batch-modify` command,
//! `users.messages.batchModify`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::messages::batch_modify::GmailMessagesBatchModify;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Modify the labels of several Gmail messages at once
/// (users.messages.batchModify).
#[derive(Debug, Parser)]
pub struct GmailMessageBatchModifyCommand {
    /// The ids of the messages to modify.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
    /// Label id to add to every message. Can be repeated.
    #[arg(long = "add-label", value_name = "ID")]
    pub add: Vec<String>,
    /// Label id to remove from every message. Can be repeated.
    #[arg(long = "remove-label", value_name = "ID")]
    pub remove: Vec<String>,
}

impl GmailMessageBatchModifyCommand {
    /// Applies the label changes to every named message.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let count = self.ids.len();

        {
            let c = GmailMessagesBatchModify::new(
                &client.auth,
                &client.user_id,
                &self.ids,
                &self.add,
                &self.remove,
            )?;
            client.run(c)?
        };

        printer.out(Message::new(format!("{count} messages modified")))
    }
}
