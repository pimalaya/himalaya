//! # Gmail draft get
//!
//! The `gmail drafts get` command, `users.drafts.get`.

use std::fmt;

use anyhow::{Result, anyhow};
use clap::Parser;
use io_gmail::v1::rest::messages::decode_raw;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    gmail::{
        client::GmailClient,
        format::{FormatArg, snippet_text, str_of},
    },
    shared::output::write_bytes_or_save,
};

/// Get a single Gmail draft (users.drafts.get).
///
/// JSON output: the draft as Gmail sent it, or under `--format raw` the
/// message it carries, as bytes.
#[derive(Debug, Parser)]
pub struct GmailDraftGetCommand {
    /// The id of the draft to get.
    #[arg(value_name = "ID")]
    pub id: String,
    /// The amount of message detail to return.
    #[arg(long, value_enum, default_value_t)]
    pub format: FormatArg,
}

impl GmailDraftGetCommand {
    /// Fetches the draft and prints it at the requested detail.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let query = self.format.query(&[]);
        let draft = client.raw_get_with(&["drafts", &self.id], query)?;

        let raw = draft
            .get("message")
            .and_then(|message| message.get("raw"))
            .and_then(Value::as_str);

        if let (FormatArg::Raw, Some(raw)) = (self.format, raw) {
            let bytes =
                decode_raw(raw).map_err(|err| anyhow!("Decode Gmail draft error: {err}"))?;
            return write_bytes_or_save(printer, None, bytes, "message/rfc822");
        }

        printer.out(GmailDraftGetOutput(Value::Object(draft)))
    }
}

/// One Gmail draft as Gmail sent it, every property kept.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailDraftGetOutput(#[schemars(with = "Map<String, Value>")] Value);

impl fmt::Display for GmailDraftGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();
        fields.push("Draft id", sanitize::text(str_of(&self.0, "id")));

        if let Some(message) = self.0.get("message") {
            fields.push("Message id", sanitize::text(str_of(message, "id")));
            if let Some(thread_id) = message.get("threadId").and_then(Value::as_str) {
                fields.push("Thread", sanitize::text(thread_id));
            }
            if let Some(snippet) = message.get("snippet").and_then(Value::as_str) {
                fields.push("Snippet", sanitize::text(&snippet_text(snippet)));
            }
        }

        write!(f, "{fields}")
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::gmail::{client::GmailClient, drafts::get::GmailDraftGetCommand, format::FormatArg};

    /// The draft prints as Gmail sent it, a property io-gmail does not
    /// model kept.
    #[test]
    fn a_draft_prints_as_gmail_sent_it() {
        let message = json!({"id": "m1", "threadId": "t1", "labelIds": ["DRAFT"]});
        let draft = json!({"id": "d1", "x-vendor": 1, "message": message});
        let (mut client, requests) = GmailClient::scripted(std::slice::from_ref(&draft));
        let mut printer = CapturePrinter::default();

        let command = GmailDraftGetCommand {
            id: "d1".into(),
            format: FormatArg::Metadata,
        };
        command.execute(&mut printer, &mut client).unwrap();

        assert_eq!(printer.into_single().unwrap(), draft);
        let requests = requests.lock().unwrap();
        let line = "GET /gmail/v1/users/me/drafts/d1?format=metadata ";
        assert!(requests[0].starts_with(line), "{}", requests[0]);
    }
}
