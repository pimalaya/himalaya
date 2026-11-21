//! # Gmail message get
//!
//! The `gmail messages get` command, `users.messages.get`.

use std::fmt;

use anyhow::{Result, anyhow};
use clap::Parser;
use io_gmail::v1::rest::messages::{
    GmailMessage, GmailMessageHeader, GmailMessagePayload, decode_raw,
};
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    gmail::{
        client::GmailClient,
        format::{FormatArg, snippet_text},
    },
    output::write_bytes_or_save,
};

/// Get a single Gmail message (users.messages.get).
///
/// JSON output: the message as Gmail sent it. Under `--format full` it
/// carries the MIME payload, bodies and attachment ids included, and the
/// text lists the parts.
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
    /// since the API honours the filter for that format alone. The JSON
    /// payload stays as Gmail sent it.
    #[arg(long = "header", value_name = "NAME")]
    pub headers: Vec<String>,
}

impl GmailMessageGetCommand {
    /// Fetches the message and prints it at the requested detail.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let query = self.format.query(&self.headers);
        let message = client.raw_get_with(&["messages", &self.id], query)?;

        if let FormatArg::Raw = self.format
            && let Some(Value::String(raw)) = message.get("raw")
        {
            let bytes =
                decode_raw(raw).map_err(|err| anyhow!("Decode Gmail message error: {err}"))?;
            return write_bytes_or_save(printer, None, bytes, "message/rfc822");
        }

        printer.out(GmailMessageGetOutput {
            message: Value::Object(message),
            headers: self.headers,
            full: matches!(self.format, FormatArg::Full),
        })
    }
}

/// The `gmail messages get` output: the message as Gmail sent it, every
/// property kept, its MIME payload under `--format full`.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailMessageGetOutput {
    #[schemars(with = "Map<String, Value>")]
    message: Value,
    /// The headers the text form shows, all of them when empty.
    #[serde(skip)]
    headers: Vec<String>,
    /// Whether the text form lists the parts (`--format full`).
    #[serde(skip)]
    full: bool,
}

impl fmt::Display for GmailMessageGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg: GmailMessage = serde_json::from_value(self.message.clone()).unwrap_or_default();
        let names: Vec<&str> = self.headers.iter().map(String::as_str).collect();
        let headers = message_headers(msg.payload.clone(), &names);
        let labels: Vec<String> = headers
            .iter()
            .map(|header| sanitize::text(&header.name).into_owned())
            .collect();

        let mut fields = Fields::default();
        fields.push("Id", sanitize::text(&msg.id));
        if let Some(thread_id) = &msg.thread_id {
            fields.push("Thread", sanitize::text(thread_id));
        }
        if !msg.label_ids.is_empty() {
            fields.push("Labels", sanitize::text(&msg.label_ids.join(", ")));
        }
        if let Some(snippet) = &msg.snippet {
            fields.push("Snippet", sanitize::text(&snippet_text(snippet)));
        }
        if let Some(size) = msg.size_estimate {
            fields.push("Size", size);
        }
        if let Some(internal_date) = &msg.internal_date {
            fields.push("Internal date", sanitize::text(internal_date));
        }

        for (label, header) in labels.iter().zip(&headers) {
            fields.push(label, sanitize::text(&header.value));
        }

        let parts = match (&msg.payload, self.full) {
            (Some(payload), true) => leaf_parts(payload),
            _ => Vec::new(),
        };
        for (label, value) in &parts {
            fields.push(label, value);
        }

        write!(f, "{fields}")
    }
}

/// The `(label, value)` rows of a payload's leaf parts, the attachment id
/// among them being what `gmail attachments get` takes.
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
/// Filtered here since the full format ignores `metadataHeaders`; matching
/// is case-insensitive, order and repeats kept. Only the top-level part
/// holds the message headers, nested ones carry their own MIME ones.
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

    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use super::{GmailMessageGetCommand, GmailMessageGetOutput, message_headers};
    use crate::gmail::{client::GmailClient, format::FormatArg};

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
        let message = json!({"id": "m1", "payload": payload});
        let output = GmailMessageGetOutput {
            message,
            headers: Vec::new(),
            full: true,
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

    /// The message prints as sent, unmodelled properties and headers kept,
    /// the headers asked for narrowing the metadata request.
    #[test]
    fn a_message_prints_as_gmail_sent_it() {
        let headers = json!([{"name": "Subject", "value": "hi"}, {"name": "To", "value": "b"}]);
        let message = json!({"id": "m1", "x-vendor": 1, "payload": {"headers": headers}});
        let (mut client, requests) = GmailClient::scripted(std::slice::from_ref(&message));
        let mut printer = CapturePrinter::default();

        let command = GmailMessageGetCommand {
            id: "m1".into(),
            format: FormatArg::Metadata,
            headers: vec!["Subject".into()],
        };
        command.execute(&mut printer, &mut client).unwrap();

        assert_eq!(printer.into_single().unwrap(), message);
        let requests = requests.lock().unwrap();
        let line = "GET /gmail/v1/users/me/messages/m1?format=metadata&metadataHeaders=Subject ";
        assert!(requests[0].starts_with(line), "{}", requests[0]);
    }
}
