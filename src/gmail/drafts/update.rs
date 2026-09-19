//! # Gmail draft update
//!
//! The `gmail drafts update` command, `users.drafts.update`.

use anyhow::{Result, anyhow};
use clap::Parser;
use io_gmail::v1::rest::{
    drafts::{GmailDraft, update::GmailDraftUpdate},
    messages::{GmailMessage, encode_raw},
};
use pimalaya_cli::printer::Printer;

use crate::{
    gmail::{client::GmailClient, drafts::create::GmailDraftWriteOutput},
    shared::message::arg::MessageArg,
};

/// Update a Gmail draft (users.drafts.update).
///
/// JSON output contains `id`, `message-id` and nullable `thread-id`.
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

        let message = draft
            .message
            .ok_or_else(|| anyhow!("Gmail draft response has no message"))?;
        printer.out(GmailDraftWriteOutput {
            id: draft.id,
            message_id: message.id,
            thread_id: message.thread_id,
            action: "updated",
        })
    }
}
