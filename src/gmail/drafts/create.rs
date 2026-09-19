//! # Gmail draft create
//!
//! The `gmail drafts create` command, `users.drafts.create`.

use core::fmt;

use anyhow::{Result, anyhow};
use clap::Parser;
use io_gmail::v1::rest::{
    drafts::{GmailDraft, create::GmailDraftCreate},
    messages::{GmailMessage, encode_raw},
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{gmail::client::GmailClient, shared::message::arg::MessageArg};

/// Create a Gmail draft (users.drafts.create).
///
/// JSON output contains `id`, `message-id` and nullable `thread-id`.
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

        let message = draft
            .message
            .ok_or_else(|| anyhow!("Gmail draft response has no message"))?;
        printer.out(GmailDraftWriteOutput {
            id: draft.id,
            message_id: message.id,
            thread_id: message.thread_id,
            action: "created",
        })
    }
}

/// Identity of a created or replaced Gmail draft.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailDraftWriteOutput {
    /// The immutable draft id.
    pub(crate) id: String,
    /// The id of the message currently stored in the draft.
    pub(crate) message_id: String,
    /// The thread id returned by Gmail, when present.
    pub(crate) thread_id: Option<String>,
    /// The verb used only in the text confirmation.
    #[serde(skip)]
    pub(crate) action: &'static str,
}

impl fmt::Display for GmailDraftWriteOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gmail draft `{}` successfully {}", self.id, self.action)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json, to_value};

    use crate::{gmail::drafts::create::GmailDraftWriteOutput, json_schema::schemas};

    #[test]
    fn draft_write_output_preserves_identities_and_text() {
        for action in ["created", "updated"] {
            for thread_id in [Some("thread-1".to_owned()), None] {
                let output = GmailDraftWriteOutput {
                    id: "draft-1".to_owned(),
                    message_id: "message-2".to_owned(),
                    thread_id: thread_id.clone(),
                    action,
                };
                assert_eq!(
                    to_value(&output).unwrap(),
                    json!({"id": "draft-1", "message-id": "message-2", "thread-id": thread_id})
                );
                assert_eq!(
                    output.to_string(),
                    format!("Gmail draft `draft-1` successfully {action}")
                );
            }
        }
    }

    #[test]
    fn draft_write_schemas_describe_the_json_output() {
        let schemas = schemas();
        let create = &schemas["himalaya-gmail-drafts-create"];
        assert_eq!(create, &schemas["himalaya-gmail-drafts-update"]);
        let properties = create["properties"].as_object().unwrap();
        assert_eq!(properties.len(), 3);
        assert_eq!(properties["id"]["type"], "string");
        assert_eq!(properties["message-id"]["type"], "string");
        assert_eq!(properties["thread-id"]["type"], json!(["string", "null"]));
        let required = create["required"].as_array().unwrap();
        assert!(required.contains(&Value::from("id")));
        assert!(required.contains(&Value::from("message-id")));
    }
}
