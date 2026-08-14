use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::drafts::{GmailDraft, send::GmailDraftSend};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Send a Gmail draft (users.drafts.send).
#[derive(Debug, Parser)]
pub struct GmailDraftSendCommand {
    /// The id of the draft to send.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailDraftSendCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let draft = GmailDraft {
            id: self.id.clone(),
            message: None,
        };

        let message_id = {
            let c = GmailDraftSend::new(&client.auth, &client.user_id, &draft)?;
            client.run(c)?
        }
        .response;

        printer.out(Message::new(format!(
            "Gmail draft sent as message `{}`",
            message_id.id
        )))
    }
}
