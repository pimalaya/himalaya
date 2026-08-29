//! # Gmail draft create
//!
//! The `gmail drafts create` command, `users.drafts.create`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::{
    drafts::{GmailDraft, create::GmailDraftCreate},
    messages::{GmailMessage, encode_raw},
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{gmail::client::GmailClient, shared::message::arg::MessageArg};

/// Create a Gmail draft (users.drafts.create).
#[derive(Debug, Parser)]
pub struct GmailDraftCreateCommand {
    /// Thread id to attach the draft to.
    #[arg(long = "thread-id", value_name = "ID")]
    pub thread_id: Option<String>,
    #[command(flatten)]
    pub message: MessageArg,
}

impl GmailDraftCreateCommand {
    /// Creates the draft and reports its new id.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();

        let draft = GmailDraft {
            id: String::new(),
            message: Some(GmailMessage {
                raw: Some(encode_raw(&raw)),
                thread_id: self.thread_id.clone(),
                ..Default::default()
            }),
        };

        let draft = {
            let c = GmailDraftCreate::new(&client.auth, &client.user_id, &draft)?;
            client.run(c)?
        }
        .response;

        printer.out(Message::new(format!(
            "Gmail draft `{}` successfully created",
            draft.id
        )))
    }
}
