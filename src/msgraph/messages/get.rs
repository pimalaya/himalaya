//! # Microsoft Graph message get
//!
//! The `msgraph messages get` command, fetching one message or its raw
//! MIME bytes.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::rest::users::messages::MsgraphMessage;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

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
            return write_bytes_or_save(printer, None, &bytes);
        }

        let msg = client.message_get(&self.id)?.response;

        printer.out(MsgraphMessageGetOutput(msg))
    }
}

/// The `msgraph messages get` output, one message.
///
/// The resource is emitted verbatim, so a message has the very same shape
/// here as in a `list` row. The text stays a summary, `--raw` being how
/// the RFC 5322 bytes are reached.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct MsgraphMessageGetOutput(MsgraphMessage);

impl fmt::Display for MsgraphMessageGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Id: {}", self.0.id)?;

        if let Some(subject) = &self.0.subject {
            writeln!(f, "Subject: {subject}")?;
        }
        if let Some(from) = self.0.from.as_ref().map(recipient) {
            writeln!(f, "From: {from}")?;
        }

        let to: Vec<String> = self.0.to_recipients.iter().map(recipient).collect();
        if !to.is_empty() {
            writeln!(f, "To: {}", to.join(", "))?;
        }

        if let Some(date) = &self.0.received_date_time {
            writeln!(f, "Received: {date}")?;
        }
        if let Some(is_read) = self.0.is_read {
            writeln!(f, "Read: {is_read}")?;
        }
        if let Some(folder) = &self.0.parent_folder_id {
            writeln!(f, "Folder: {folder}")?;
        }
        if let Some(preview) = &self.0.body_preview {
            writeln!(f, "Preview: {preview}")?;
        }

        Ok(())
    }
}
