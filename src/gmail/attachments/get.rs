use std::path::PathBuf;

use anyhow::{Result, anyhow};
use clap::Parser;
use io_gmail::v1::rest::messages::{attachments::get::GmailAttachmentGet, decode_raw};
use pimalaya_cli::printer::Printer;

use crate::{gmail::client::GmailClient, shared::output::write_bytes_or_save};

/// Get a Gmail attachment by message and attachment id, then print or
/// save its decoded bytes.
#[derive(Debug, Parser)]
pub struct GmailAttachmentGetCommand {
    /// Identifier of the message owning the attachment.
    #[arg(value_name = "MESSAGE_ID")]
    pub message_id: String,
    /// Identifier of the attachment to get.
    #[arg(value_name = "ATTACHMENT_ID")]
    pub id: String,
    /// Save the decoded bytes to this path instead of printing them.
    #[arg(short = 'o', long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

impl GmailAttachmentGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c =
                GmailAttachmentGet::new(&client.auth, &client.user_id, &self.message_id, &self.id)?;
            client.run(c)?
        };
        let body = out.response;

        let data = body
            .data
            .ok_or_else(|| anyhow!("Gmail attachment has no data"))?;
        let bytes =
            decode_raw(&data).map_err(|err| anyhow!("Decode Gmail attachment error: {err}"))?;

        write_bytes_or_save(printer, self.output.as_deref(), &bytes)
    }
}
