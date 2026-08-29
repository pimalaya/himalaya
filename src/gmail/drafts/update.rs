//! # Gmail draft update
//!
//! The `gmail drafts update` command, `users.drafts.update`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::{
    drafts::{GmailDraft, update::GmailDraftUpdate},
    messages::{GmailMessage, encode_raw},
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{gmail::client::GmailClient, shared::message::arg::MessageArg};

/// Update a Gmail draft (users.drafts.update).
#[derive(Debug, Parser)]
pub struct GmailDraftUpdateCommand {
    /// The id of the draft to update.
    #[arg(value_name = "ID")]
    pub id: String,
    /// Thread id to attach the draft to.
    #[arg(long = "thread-id", value_name = "ID")]
    pub thread_id: Option<String>,
    #[command(flatten)]
    pub message: MessageArg,
}

impl GmailDraftUpdateCommand {
    /// Replaces the draft with the given message.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();

        let draft = GmailDraft {
            id: self.id.clone(),
            message: Some(GmailMessage {
                raw: Some(encode_raw(&raw)),
                thread_id: self.thread_id.clone(),
                ..Default::default()
            }),
        };

        let draft = {
            let c = GmailDraftUpdate::new(&client.auth, &client.user_id, &draft)?;
            client.run(c)?
        }
        .response;

        printer.out(Message::new(format!(
            "Gmail draft `{}` successfully updated",
            draft.id
        )))
    }
}
