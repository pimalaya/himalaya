use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::msgraph::client::MsgraphClient;

/// Permanently delete a Microsoft Graph message (`DELETE
/// /me/messages/{id}`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageDeleteCommand {
    /// The id of the message to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl MsgraphMessageDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        client.message_delete(&self.id)?;
        printer.out(Message::new(format!(
            "Microsoft Graph message `{}` permanently deleted",
            self.id
        )))
    }
}
