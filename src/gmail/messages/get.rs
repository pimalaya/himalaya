//! # Gmail message get
//!
//! The `gmail messages get` command, `users.messages.get`.

use std::fmt;

use anyhow::{Result, anyhow};
use clap::Parser;
use io_gmail::v1::rest::messages::{
    GmailMessageFormat, GmailMessageHeader, GmailMessagePayload, decode_raw,
};
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
    /// Only render the given header. Can be repeated, and matched
    /// case-insensitively.
    ///
    /// Under `--format metadata` it also narrows what Gmail sends back,
    /// since the API honours the filter for that format alone.
    #[arg(long = "header", value_name = "NAME")]
    pub headers: Vec<String>,
}

impl GmailMessageGetCommand {
    /// Fetches the message and prints it at the requested detail.
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

        printer.out(GmailMessageGetOutput {
            id: msg.id,
            thread_id: msg.thread_id,
            label_ids: msg.label_ids,
            snippet: msg.snippet,
            size_estimate: msg.size_estimate,
            internal_date: msg.internal_date,
            headers: message_headers(msg.payload, &hs),
        })
    }
}

/// The `gmail messages get` output, the metadata of one message.
///
/// The whole resource is never exposed: a full-format payload is a
/// recursive MIME tree of base64 bodies, which belongs on the raw path.
/// `--format raw` and `attachments get` are how a body is reached.
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

/// One RFC 5322 header of a Gmail message.
///
/// Headers are a list rather than a map, a message being free to repeat a
/// name and their order being meaningful.
#[derive(Serialize, JsonSchema)]
pub(crate) struct GmailMessageHeaderOutput {
    /// The header name.
    pub name: String,
    /// Its value.
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

/// Folds the headers of a payload into output headers, keeping the names
/// `--header` asked for and all of them when it asked for none.
///
/// The filter runs here because Gmail honours `metadataHeaders` under the
/// metadata format alone, the full format returning every header whatever
/// was asked for. Matching is case-insensitive, and both the order and
/// the repeats survive.
///
/// Only the top-level part is read, which is where Gmail puts the message
/// headers, a nested part carrying its own MIME ones.
pub(crate) fn message_headers(
    payload: Option<GmailMessagePayload>,
    names: &[&str],
) -> Vec<GmailMessageHeaderOutput> {
    payload
        .map(|payload| payload.headers)
        .unwrap_or_default()
        .into_iter()
        .filter(|header| {
            names.is_empty()
                || names
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(&header.name))
        })
        .map(GmailMessageHeaderOutput::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use io_gmail::v1::rest::messages::{GmailMessageHeader, GmailMessagePayload};

    use super::message_headers;

    fn payload(headers: &[(&str, &str)]) -> Option<GmailMessagePayload> {
        Some(GmailMessagePayload {
            headers: headers
                .iter()
                .map(|(name, value)| GmailMessageHeader {
                    name: name.to_string(),
                    value: value.to_string(),
                })
                .collect(),
            ..Default::default()
        })
    }

    #[test]
    fn no_requested_name_keeps_every_header() {
        let headers = message_headers(payload(&[("Subject", "hi"), ("From", "a@b")]), &[]);

        let names: Vec<_> = headers.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(names, ["Subject", "From"]);
    }

    #[test]
    fn requested_names_match_case_insensitively() {
        let headers = message_headers(
            payload(&[("Subject", "hi"), ("From", "a@b"), ("To", "c@d")]),
            &["subject", "TO"],
        );

        let names: Vec<_> = headers.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(names, ["Subject", "To"]);
    }

    #[test]
    fn repeated_headers_keep_their_order_and_duplicates() {
        let headers = message_headers(
            payload(&[("Received", "one"), ("Subject", "hi"), ("Received", "two")]),
            &["Received"],
        );

        let values: Vec<_> = headers.iter().map(|h| h.value.as_str()).collect();
        assert_eq!(values, ["one", "two"]);
    }

    #[test]
    fn a_payload_less_message_has_no_header() {
        assert!(message_headers(None, &["Subject"]).is_empty());
    }
}
