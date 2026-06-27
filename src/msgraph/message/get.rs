use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    msgraph::{client::MsgraphClient, message::list::recipient},
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
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        if self.raw {
            let bytes = client.message_get_raw(&self.id)?.response;
            return write_bytes_or_save(printer, None, &bytes);
        }

        let msg = client.message_get(&self.id)?.response;

        let mut out = String::new();
        out.push_str(&format!("Id: {}\n", msg.id));
        if let Some(subject) = &msg.subject {
            out.push_str(&format!("Subject: {subject}\n"));
        }
        if let Some(from) = msg.from.as_ref().map(recipient) {
            out.push_str(&format!("From: {from}\n"));
        }
        let to: Vec<String> = msg.to_recipients.iter().map(recipient).collect();
        if !to.is_empty() {
            out.push_str(&format!("To: {}\n", to.join(", ")));
        }
        if let Some(date) = &msg.received_date_time {
            out.push_str(&format!("Received: {date}\n"));
        }
        if let Some(is_read) = msg.is_read {
            out.push_str(&format!("Read: {is_read}\n"));
        }
        if let Some(folder) = &msg.parent_folder_id {
            out.push_str(&format!("Folder: {folder}\n"));
        }
        if let Some(preview) = &msg.body_preview {
            out.push_str(&format!("Preview: {preview}\n"));
        }

        printer.out(Message::new(out))
    }
}
