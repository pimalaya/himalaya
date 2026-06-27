use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::msgraph::client::MsgraphClient;

/// Copy a Microsoft Graph message into another folder (`POST
/// /me/messages/{id}/copy`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageCopyCommand {
    /// The id of the message to copy.
    #[arg(value_name = "ID")]
    pub id: String,

    /// The destination folder id or well-known name.
    #[arg(value_name = "DESTINATION")]
    pub destination: String,
}

impl MsgraphMessageCopyCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let message = client.message_copy(&self.id, &self.destination)?.response;
        printer.out(Message::new(format!(
            "Microsoft Graph message copied to `{}`",
            message.id
        )))
    }
}
