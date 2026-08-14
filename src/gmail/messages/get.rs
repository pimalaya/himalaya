use std::fmt;

use anyhow::{Result, anyhow};
use clap::Parser;
use io_gmail::v1::rest::messages::{GmailMessageFormat, GmailMessageHeader, decode_raw};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    gmail::{client::GmailClient, format::FormatArg},
    shared::output::write_bytes_or_save,
};

/// Get a single Gmail message (users.messages.get).
#[derive(Debug, Parser)]
pub struct GmailMessageGetCommand {
    /// The id of the message to get.
    #[arg(value_name = "ID")]
    pub id: String,
    /// The amount of message detail to return.
    #[arg(long, value_enum, default_value_t)]
    pub format: FormatArg,
    /// Header to include when `--format metadata` is used. Can be
    /// repeated.
    #[arg(long = "header", value_name = "NAME")]
    pub headers: Vec<String>,
}

impl GmailMessageGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let format = GmailMessageFormat::from(self.format);
        let hs: Vec<&str> = self.headers.iter().map(String::as_str).collect();

        let msg = client.message_get(&self.id, format, &hs)?.response;

        if format == GmailMessageFormat::Raw
            && let Some(raw) = &msg.raw
        {
            let bytes =
                decode_raw(raw).map_err(|err| anyhow!("Decode Gmail message error: {err}"))?;
            return write_bytes_or_save(printer, None, &bytes);
        }

        let headers = msg
            .payload
            .map(|payload| payload.headers)
            .unwrap_or_default();

        printer.out(GmailMessageGetOutput {
            id: msg.id,
            thread_id: msg.thread_id,
            label_ids: msg.label_ids,
            snippet: msg.snippet,
            size_estimate: msg.size_estimate,
            internal_date: msg.internal_date,
            headers: headers
                .into_iter()
                .map(GmailMessageHeaderOutput::from)
                .collect(),
        })
    }
}

/// Gmail message metadata, rendered as aligned text or, under `--json`,
/// as a structured object instead of a wrapped human string.
///
/// Only the metadata is exposed, never the whole Gmail resource: the
/// payload of a message fetched with the full format is a recursive MIME
/// tree carrying base64-encoded bodies, which belongs on the `--format
/// raw` path rather than in a `--json` object. Bodies stay reachable
/// through `messages get --format raw` and `attachments get`.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailMessageGetOutput {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_id: Option<String>,
    label_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_estimate: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    internal_date: Option<String>,
    headers: Vec<GmailMessageHeaderOutput>,
}

impl fmt::Display for GmailMessageGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Id: {}", self.id)?;
        if let Some(thread_id) = &self.thread_id {
            writeln!(f, "Thread: {thread_id}")?;
        }
        if !self.label_ids.is_empty() {
            writeln!(f, "Labels: {}", self.label_ids.join(", "))?;
        }
        if let Some(snippet) = &self.snippet {
            writeln!(f, "Snippet: {snippet}")?;
        }
        if let Some(size) = self.size_estimate {
            writeln!(f, "Size: {size}")?;
        }
        if let Some(internal_date) = &self.internal_date {
            writeln!(f, "Internal date: {internal_date}")?;
        }

        for header in &self.headers {
            writeln!(f, "{}: {}", header.name, header.value)?;
        }

        Ok(())
    }
}

/// A single RFC 5322 header of a Gmail message.
///
/// Headers are a list rather than a map because a message may repeat a
/// name (Received, References) and their order is meaningful. They are
/// absent with the minimal format, restricted to the names passed to
/// `--header` with the metadata format, and complete otherwise.
#[derive(Serialize, JsonSchema)]
pub(crate) struct GmailMessageHeaderOutput {
    pub name: String,
    pub value: String,
}

impl From<GmailMessageHeader> for GmailMessageHeaderOutput {
    fn from(header: GmailMessageHeader) -> Self {
        Self {
            name: header.name,
            value: header.value,
        }
    }
}
