use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::msgraph::client::MsgraphClient;

/// Delete an attachment (`DELETE
/// /me/messages/{id}/attachments/{aid}`).
#[derive(Debug, Parser)]
pub struct MsgraphAttachmentDeleteCommand {
    /// The id of the message the attachment belongs to.
    #[arg(value_name = "MESSAGE_ID")]
    pub message_id: String,

    /// The id of the attachment to delete.
    #[arg(value_name = "ATTACHMENT_ID")]
    pub id: String,
}

impl MsgraphAttachmentDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        client.attachment_delete(&self.message_id, &self.id)?;
        printer.out(Message::new(format!(
            "Microsoft Graph attachment `{}` successfully deleted",
            self.id
        )))
    }
}
