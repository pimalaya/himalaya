use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Remove a Gmail message from the trash (users.messages.untrash).
#[derive(Debug, Parser)]
pub struct GmailMessageUntrashCommand {
    /// The id of the message to untrash.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailMessageUntrashCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let message = client.message_untrash(&self.id)?.response;
        printer.out(Message::new(format!(
            "Gmail message `{}` successfully untrashed",
            message.id
        )))
    }
}
