use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Modify the labels of a Gmail message (users.messages.modify).
#[derive(Debug, Parser)]
pub struct GmailMessageModifyCommand {
    /// The id of the message to modify.
    #[arg(value_name = "ID")]
    pub id: String,
    /// Label id to add to the message. Can be repeated.
    #[arg(long = "add-label", value_name = "ID")]
    pub add: Vec<String>,
    /// Label id to remove from the message. Can be repeated.
    #[arg(long = "remove-label", value_name = "ID")]
    pub remove: Vec<String>,
}

impl GmailMessageModifyCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let message = client
            .message_modify(&self.id, &self.add, &self.remove)?
            .response;
        printer.out(Message::new(format!(
            "Gmail message `{}` successfully modified",
            message.id
        )))
    }
}
