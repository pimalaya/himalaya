//! # Microsoft Graph message get
//!
//! The `msgraph messages get` command, fetching one message or its raw
//! MIME bytes.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::rest::users::messages::MsgraphMessage;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    msgraph::{client::MsgraphClient, messages::list::recipient},
    shared::output::write_bytes_or_save,
};

/// Get a single Microsoft Graph message (`GET /me/messages/{id}`), or its
/// raw RFC 5322 bytes with `--raw` (`GET /me/messages/{id}/$value`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageGetCommand {
    /// The id of the message to get.
    #[arg(value_name = "ID")]
    pub id: String,
    /// Return the raw RFC 5322 MIME message instead of the parsed
    /// fields.
    #[arg(long)]
    pub raw: bool,
}

impl MsgraphMessageGetCommand {
    /// Fetches the message, or its raw MIME bytes, and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        if self.raw {
            let bytes = client.message_get_raw(&self.id)?.response;
            return write_bytes_or_save(printer, None, bytes, "message/rfc822");
        }

        let message = client.raw_get(&["messages", &self.id])?;
        printer.out(MsgraphMessageGetOutput(message))
    }
}

/// The `msgraph messages get` output, one message.
///
/// The resource is emitted as Graph sent it, every property kept, as a
/// `list` row is. The text stays a summary, `--raw` being how
/// the RFC 5322 bytes are reached.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct MsgraphMessageGetOutput(#[schemars(with = "Map<String, Value>")] Value);

impl fmt::Display for MsgraphMessageGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg: MsgraphMessage = serde_json::from_value(self.0.clone()).unwrap_or_default();
        let mut fields = Fields::default();
        fields.push("Id", sanitize::text(&msg.id));

        if let Some(subject) = &msg.subject {
            fields.push("Subject", sanitize::text(subject));
        }
        if let Some(from) = msg.from.as_ref().map(recipient) {
            fields.push("From", sanitize::text(&from));
        }

        let to = msg.to_recipients.as_option().into_iter().flatten();
        let to: Vec<String> = to.map(recipient).collect();
        if !to.is_empty() {
            fields.push("To", sanitize::text(&to.join(", ")));
        }

        if let Some(date) = &msg.received_date_time {
            fields.push("Received", date);
        }
        if let Some(is_read) = msg.is_read {
            fields.push("Read", is_read);
        }
        if let Some(folder) = &msg.parent_folder_id {
            fields.push("Folder", sanitize::text(folder));
        }
        if let Some(preview) = &msg.body_preview {
            fields.push("Preview", sanitize::text(preview));
        }

        write!(f, "{fields}")
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::msgraph::{client::MsgraphClient, messages::get::MsgraphMessageGetCommand};

    /// The message prints as Graph sent it, a property io-msgraph does not
    /// model and the OData annotations kept.
    #[test]
    fn a_message_prints_as_graph_sent_it() {
        let message = json!({
            "@odata.etag": "W/\"1\"",
            "id": "M1",
            "subject": "Hi",
            "x-vendor": 1,
        });
        let (mut client, requests) = MsgraphClient::scripted(std::slice::from_ref(&message));
        let mut printer = CapturePrinter::default();

        let command = MsgraphMessageGetCommand {
            id: "M1".into(),
            raw: false,
        };
        command.execute(&mut printer, &mut client).unwrap();

        assert_eq!(printer.into_single().unwrap(), message);
        let requests = requests.lock().unwrap();
        assert!(
            requests[0].starts_with("GET /v1.0/me/messages/M1 "),
            "{}",
            requests[0]
        );
    }
}
