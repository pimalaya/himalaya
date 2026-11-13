//! # Gmail draft get
//!
//! The `gmail drafts get` command, `users.drafts.get`.

use std::fmt;

use anyhow::{Result, anyhow};
use clap::Parser;
use io_gmail::v1::rest::{
    drafts::get::GmailDraftGet,
    messages::{GmailMessageFormat, decode_raw},
};
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    gmail::{
        client::GmailClient,
        format::{FormatArg, snippet_text},
    },
    shared::output::write_bytes_or_save,
};

/// Get a single Gmail draft (users.drafts.get).
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
        let format = GmailMessageFormat::from(self.format);

        let draft = {
            let c = GmailDraftGet::new(&client.auth, &client.user_id, &self.id, format)?;
            client.run(c)?
        }
        .response;

        if format == GmailMessageFormat::Raw
            && let Some(raw) = draft
                .message
                .as_ref()
                .and_then(|message| message.raw.as_ref())
        {
            let bytes =
                decode_raw(raw).map_err(|err| anyhow!("Decode Gmail draft error: {err}"))?;
            return write_bytes_or_save(printer, None, bytes, "message/rfc822");
        }

        printer.out(GmailDraftGetOutput {
            id: draft.id,
            message: draft.message.map(|message| GmailDraftMessageOutput {
                id: message.id,
                thread_id: message.thread_id,
                snippet: message.snippet,
            }),
        })
    }
}

/// Gmail draft, rendered as aligned text or, under `--json`, as a
/// structured object instead of a wrapped human string.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailDraftGetOutput {
    id: String,
    message: Option<GmailDraftMessageOutput>,
}

impl fmt::Display for GmailDraftGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();
        fields.push("Draft id", sanitize::text(&self.id));

        if let Some(message) = &self.message {
            fields.push("Message id", sanitize::text(&message.id));
            if let Some(thread_id) = &message.thread_id {
                fields.push("Thread", sanitize::text(thread_id));
            }
            if let Some(snippet) = &message.snippet {
                fields.push("Snippet", sanitize::text(&snippet_text(snippet)));
            }
        }

        write!(f, "{fields}")
    }
}

/// The message a Gmail draft carries.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailDraftMessageOutput {
    id: String,
    thread_id: Option<String>,
    snippet: Option<String>,
}
