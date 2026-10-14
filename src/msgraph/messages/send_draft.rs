//! # Microsoft Graph message send-draft
//!
//! The `msgraph messages send-draft` command, sending an existing draft.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::msgraph::client::MsgraphClient;

/// Send an existing Microsoft Graph draft (`POST
/// /me/messages/{id}/send`); Graph moves it to Sent Items.
#[derive(Debug, Parser)]
pub struct MsgraphMessageSendDraftCommand {
    /// The id of the draft to send.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl MsgraphMessageSendDraftCommand {
    /// Sends the draft.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        client.message_send(&self.id)?;
        printer.out(Message::new(format!(
            "Microsoft Graph draft `{}` successfully sent",
            self.id
        )))
    }
}
