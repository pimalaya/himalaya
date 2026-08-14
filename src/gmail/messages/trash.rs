use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Move a Gmail message to the trash (users.messages.trash).
#[derive(Debug, Parser)]
pub struct GmailMessageTrashCommand {
    /// The id of the message to trash.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailMessageTrashCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let message = client.message_trash(&self.id)?.response;
        printer.out(Message::new(format!(
            "Gmail message `{}` successfully trashed",
            message.id
        )))
    }
}
