//! # Gmail message get
//!
//! The `gmail messages get` command, `users.messages.get`.

use std::fmt;

use anyhow::{Result, anyhow};
use clap::Parser;
use io_gmail::v1::rest::messages::{
    GmailMessageFormat, GmailMessageHeader, GmailMessagePayload, decode_raw,
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

/// Get a single Gmail message (users.messages.get).
///
/// Under `--format full`, JSON carries the MIME payload, bodies and
/// attachment ids included, and the text lists the parts.
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
            return write_bytes_or_save(printer, None, bytes, "message/rfc822");
        }

        printer.out(GmailMessageGetOutput {
            id: msg.id,
            thread_id: msg.thread_id,
            label_ids: msg.label_ids,
            snippet: msg.snippet,
            size_estimate: msg.size_estimate,
            internal_date: msg.internal_date,
            headers: message_headers(msg.payload.clone(), &hs),
            payload: msg.payload.filter(|_| format == GmailMessageFormat::Full),
        })
    }
}

/// The `gmail messages get` output, the metadata of one message, and its
/// MIME payload under `--format full`: the parts, their bodies and the
/// attachment ids `gmail attachments get` takes, keys in Gmail's spelling.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailMessageGetOutput {
    id: String,
    thread_id: Option<String>,
    label_ids: Vec<String>,
    snippet: Option<String>,
    size_estimate: Option<u64>,
    internal_date: Option<String>,
    headers: Vec<GmailMessageHeaderOutput>,
    payload: Option<GmailMessagePayload>,
}

impl fmt::Display for GmailMessageGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();
        fields.push("Id", sanitize::text(&self.id));
        if let Some(thread_id) = &self.thread_id {
            fields.push("Thread", sanitize::text(thread_id));
        }
        if !self.label_ids.is_empty() {
            fields.push("Labels", sanitize::text(&self.label_ids.join(", ")));
        }
        if let Some(snippet) = &self.snippet {
            fields.push("Snippet", sanitize::text(&snippet_text(snippet)));
        }
        if let Some(size) = self.size_estimate {
            fields.push("Size", size);
        }
        if let Some(internal_date) = &self.internal_date {
            fields.push("Internal date", sanitize::text(internal_date));
        }

        for header in &self.headers {
            fields.push(&header.name, sanitize::text(&header.value));
        }

        let parts = self.payload.as_ref().map(leaf_parts).unwrap_or_default();
        for (label, value) in &parts {
            fields.push(label, value);
        }

        write!(f, "{fields}")
    }
}

/// The `(label, value)` rows of the leaf parts of a payload, in order:
/// the part id, then its MIME type, file name, size and the attachment id
/// `gmail attachments get` takes.
fn leaf_parts(payload: &GmailMessagePayload) -> Vec<(String, String)> {
    if !payload.parts.is_empty() {
        return payload.parts.iter().flat_map(leaf_parts).collect();
    }

    let label = match payload.part_id.as_deref() {
        Some(id) if !id.is_empty() => format!("Part {}", sanitize::text(id)),
        _ => String::from("Part"),
    };

    let mime_type = payload
        .mime_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    let mut value = vec![sanitize::text(mime_type).to_string()];
    if !payload.filename.is_empty() {
        value.push(format!("\"{}\"", sanitize::text(&payload.filename)));
    }
    if let Some(body) = &payload.body {
        value.push(format!("{} bytes", body.size));
        if let Some(id) = &body.attachment_id {
            value.push(format!("attachment {}", sanitize::text(id)));
        }
    }

    vec![(label, value.join(", "))]
}

/// One RFC 5322 header of a Gmail message.
///
/// Headers are a list rather than a map, a message being free to repeat a
/// name and their order being meaningful.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
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
    use io_gmail::v1::rest::messages::{
        GmailMessageHeader, GmailMessagePartBody, GmailMessagePayload,
    };

    use super::{GmailMessageGetOutput, message_headers};

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

    #[test]
    fn a_full_message_shows_its_parts_and_attachment_ids() {
        let part =
            |id: &str, mime: &str, filename: &str, attachment: Option<&str>| GmailMessagePayload {
                part_id: Some(id.into()),
                mime_type: Some(mime.into()),
                filename: filename.into(),
                body: Some(GmailMessagePartBody {
                    attachment_id: attachment.map(Into::into),
                    size: 12,
                    data: None,
                }),
                ..Default::default()
            };
        let payload = GmailMessagePayload {
            mime_type: Some("multipart/mixed".into()),
            parts: vec![
                part("0", "text/plain", "", None),
                part("1", "image/png", "logo.png", Some("ANGj")),
            ],
            ..Default::default()
        };
        let output = GmailMessageGetOutput {
            id: "m1".into(),
            thread_id: None,
            label_ids: Vec::new(),
            snippet: None,
            size_estimate: None,
            internal_date: None,
            headers: Vec::new(),
            payload: Some(payload),
        };

        let text = output.to_string();
        assert!(text.contains("text/plain, 12 bytes"), "{text}");
        assert!(
            text.contains("image/png, \"logo.png\", 12 bytes, attachment ANGj"),
            "{text}"
        );

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["payload"]["parts"][1]["body"]["attachmentId"], "ANGj");
    }
}
