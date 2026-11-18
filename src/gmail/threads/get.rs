//! # Gmail thread get
//!
//! The `gmail threads get` command, `users.threads.get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::messages::GmailMessage;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::gmail::{
    client::GmailClient,
    format::{FormatArg, snippet_text},
    messages::get::message_headers,
};

/// Get a single Gmail thread with all its messages
/// (users.threads.get).
///
/// JSON output: the thread as Gmail sent it, each message's MIME payload
/// included when Gmail supplies one, bodies and attachment metadata under
/// `--format full`. `--header` filters the headers of the text form, and
/// narrows the metadata request, never the JSON payload.
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
    /// Fetches the thread and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let query = self.format.query(&self.headers);
        let thread = client.raw_get_with(&["threads", &self.id], query)?;

        printer.out(GmailThreadGetOutput {
            thread: Value::Object(thread),
            headers: self.headers,
        })
    }
}

/// The `gmail threads get` output: the thread as Gmail sent it, every
/// property kept.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailThreadGetOutput {
    #[schemars(with = "Map<String, Value>")]
    thread: Value,
    /// The headers the text form shows, all of them when empty.
    #[serde(skip)]
    headers: Vec<String>,
}

impl fmt::Display for GmailThreadGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = self.thread.get("id").and_then(Value::as_str);
        let names: Vec<&str> = self.headers.iter().map(String::as_str).collect();
        let messages: Vec<GmailMessage> = self
            .thread
            .get("messages")
            .cloned()
            .and_then(|messages| serde_json::from_value(messages).ok())
            .unwrap_or_default();

        let mut thread = Fields::default();
        thread.push("Thread id", sanitize::text(id.unwrap_or_default()));
        write!(f, "{thread}")?;

        for message in &messages {
            let headers = message_headers(message.payload.clone(), &names);
            let labels: Vec<_> = headers
                .iter()
                .map(|header| sanitize::text(&header.name))
                .collect();

            let mut fields = Fields::default();
            fields.push("Message id", sanitize::text(&message.id));

            if let Some(snippet) = &message.snippet {
                fields.push("Snippet", sanitize::text(&snippet_text(snippet)));
            }

            for (label, header) in labels.iter().zip(&headers) {
                fields.push(label, sanitize::text(&header.value));
            }

            write!(f, "\n{fields}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::gmail::{
        client::GmailClient, format::FormatArg, threads::get::GmailThreadGetCommand,
    };

    /// The thread prints as Gmail sent it, a property io-gmail does not
    /// model kept, its messages unreshaped.
    #[test]
    fn a_thread_prints_as_gmail_sent_it() {
        let message = json!({"id": "m1", "threadId": "t1", "x-part": true});
        let thread = json!({"id": "t1", "historyId": "9", "x-vendor": 1, "messages": [message]});
        let (mut client, requests) = GmailClient::scripted(std::slice::from_ref(&thread));
        let mut printer = CapturePrinter::default();

        let command = GmailThreadGetCommand {
            id: "t1".into(),
            format: FormatArg::Minimal,
            headers: Vec::new(),
        };
        command.execute(&mut printer, &mut client).unwrap();

        assert_eq!(printer.into_single().unwrap(), thread);
        let requests = requests.lock().unwrap();
        let line = "GET /gmail/v1/users/me/threads/t1?format=minimal ";
        assert!(requests[0].starts_with(line), "{}", requests[0]);
    }
}
