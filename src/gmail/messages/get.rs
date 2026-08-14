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
/// absent with the minimal format, narrowed by `--header` when it is
/// given, and complete otherwise.
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

/// Folds the RFC 5322 headers of a message payload into output headers,
/// keeping only the `names` requested with `--header`. An empty `names`
/// keeps them all.
///
/// The filter is applied here rather than left to Gmail because the
/// `metadataHeaders` query parameter narrows the response under the
/// metadata format alone: the full format returns every header whatever
/// is asked for. Matching is case-insensitive, as RFC 5322 header names
/// are, and both the order and the repeats of the payload are kept.
///
/// Only the top-level part is read, since that is where Gmail puts the
/// message headers; nested parts carry their own MIME headers.
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
