//! # Gmail thread get
//!
//! The `gmail threads get` command, `users.threads.get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::{messages::GmailMessageFormat, threads::get::GmailThreadGet};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::{
    client::GmailClient,
    format::FormatArg,
    messages::get::{GmailMessageHeaderOutput, message_headers},
};

/// Get a single Gmail thread with all its messages
/// (users.threads.get).
#[derive(Debug, Parser)]
pub struct GmailThreadGetCommand {
    /// The id of the thread to get.
    #[arg(value_name = "ID")]
    pub id: String,
    /// The amount of message detail to return.
    #[arg(long, value_enum, default_value_t)]
    pub format: FormatArg,
    /// Only render the given header. Can be repeated, and matched
    /// case-insensitively.
    ///
    /// Under `--format metadata` it also narrows what Gmail sends back,
    /// since the API honours the filter for that format alone.
    #[arg(long = "header", value_name = "NAME")]
    pub headers: Vec<String>,
}

impl GmailThreadGetCommand {
    /// Fetches the thread and prints its messages.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let format = GmailMessageFormat::from(self.format);
        let hs: Vec<&str> = self.headers.iter().map(String::as_str).collect();

        let out = {
            let c = GmailThreadGet::new(&client.auth, &client.user_id, &self.id, format, &hs)?;
            client.run(c)?
        };
        let thread = out.response;

        let messages = thread
            .messages
            .into_iter()
            .map(|message| GmailThreadMessageOutput {
                id: message.id,
                label_ids: message.label_ids,
                snippet: message.snippet,
                headers: message_headers(message.payload, &hs),
            })
            .collect();

        printer.out(GmailThreadGetOutput {
            id: thread.id,
            messages,
        })
    }
}

/// Gmail thread and its messages, rendered as an indented list or,
/// under `--json`, as a structured object instead of a wrapped human
/// string.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailThreadGetOutput {
    id: String,
    messages: Vec<GmailThreadMessageOutput>,
}

impl fmt::Display for GmailThreadGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Thread id: {}", self.id)?;

        for message in &self.messages {
            let snippet = message.snippet.as_deref().unwrap_or("");
            writeln!(f, "- {}: {snippet}", message.id)?;

            for header in &message.headers {
                writeln!(f, "  {}: {}", header.name, header.value)?;
            }
        }

        Ok(())
    }
}

/// One message of a Gmail thread.
///
/// The thread id is left out, since it is the id of the enclosing
/// [`GmailThreadGetOutput`].
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailThreadMessageOutput {
    id: String,
    label_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snippet: Option<String>,
    headers: Vec<GmailMessageHeaderOutput>,
}
