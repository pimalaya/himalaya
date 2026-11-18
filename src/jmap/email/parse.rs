//! # JMAP email parse
//!
//! The `jmap email parse` command, RFC 8621 `Email/parse`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    error::{ErrorCode, ItemError, partial_when, write_item_errors},
    jmap::client::JmapClient,
};

/// Parse RFC 5322 message blobs without storing them (Email/parse).
///
/// Useful for reading attached .eml files or message blobs that are
/// not yet stored as Email objects. A blob the server does not find, or
/// cannot parse, is an item error, and the command exits 3.
#[derive(Debug, Parser)]
pub struct JmapEmailParseCommand {
    /// Blob ID(s) to parse as RFC 5322 messages.
    #[arg(value_name = "ID", required = true)]
    pub blob_ids: Vec<String>,
}

impl JmapEmailParseCommand {
    /// Parses the blobs and prints the text bodies they hold.
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let output = client.email_parse(self.blob_ids.clone(), Default::default())?;

        let mut errors = Vec::new();

        for id in output.not_found {
            errors.push(blob_error(id, ErrorCode::MessageNotFound, "Blob not found"));
        }

        for id in output.not_parsable {
            let error = "Blob is not a valid MIME message";
            errors.push(blob_error(id, ErrorCode::MessageUnreadable, error));
        }

        let mut bodies = Vec::new();

        for (blob_id, email) in output.parsed {
            if let Some(body_values) = &email.body_values
                && let Some(text_parts) = &email.text_body
            {
                for part in text_parts {
                    if let Some(part_id) = &part.part_id
                        && let Some(body_value) = body_values.get(part_id)
                    {
                        bodies.push(JmapEmailParseBody {
                            blob_id: blob_id.clone(),
                            body: body_value.value.clone(),
                        });
                    }
                }
            }
        }

        let partial = partial_when(&errors);
        printer.out(JmapEmailParseOutput { bodies, errors })?;
        partial
    }
}

/// The item error of a blob `Email/parse` could not read.
fn blob_error(id: String, code: ErrorCode, error: &str) -> ItemError {
    ItemError {
        id: Some(id),
        code: Some(code),
        error: error.to_owned(),
        sources: Vec::new(),
    }
}

/// The `jmap email parse` output, the text bodies the blobs held.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JmapEmailParseOutput {
    /// The text bodies, each beside the blob it came from, in the order
    /// the server answered.
    bodies: Vec<JmapEmailParseBody>,
    /// The item error of each blob not found or not parsable, `[]` when
    /// none.
    errors: Vec<ItemError>,
}

/// One text body of a parsed blob.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JmapEmailParseBody {
    /// The blob the body was parsed from.
    blob_id: String,
    /// The decoded text of the body.
    body: String,
}

impl fmt::Display for JmapEmailParseOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, body) in self.bodies.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }

            f.write_str(&body.body)?;

            if !body.body.ends_with('\n') {
                writeln!(f)?;
            }
        }

        write_item_errors(f, &self.errors)
    }
}

#[cfg(test)]
mod tests {
    use crate::jmap::email::parse::{JmapEmailParseBody, JmapEmailParseOutput};

    /// Each body names the blob it came from, and the errors print, `[]`
    /// when none.
    #[test]
    fn each_body_names_its_blob() {
        let output = JmapEmailParseOutput {
            bodies: vec![JmapEmailParseBody {
                blob_id: "B1".into(),
                body: "hello\n".into(),
            }],
            errors: Vec::new(),
        };

        let json = serde_json::to_value(&output).unwrap();
        let expected = serde_json::json!({
            "bodies": [{"blobId": "B1", "body": "hello\n"}],
            "errors": [],
        });
        assert_eq!(json, expected);
    }
}
