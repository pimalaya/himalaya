use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{msgraph::client::MsgraphClient, shared::output::write_bytes_or_save};

/// Download an attachment's content (`GET
/// /me/messages/{id}/attachments/{aid}/$value`), then print or save its
/// bytes.
#[derive(Debug, Parser)]
pub struct MsgraphAttachmentGetCommand {
    /// The id of the message the attachment belongs to.
    #[arg(value_name = "MESSAGE_ID")]
    pub message_id: String,

    /// The id of the attachment to download.
    #[arg(value_name = "ATTACHMENT_ID")]
    pub id: String,

    /// Save the attachment to this path instead of printing its bytes.
    #[arg(short = 'o', long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

impl MsgraphAttachmentGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let bytes = client
            .attachment_get_raw(&self.message_id, &self.id)?
            .response;

        write_bytes_or_save(printer, self.output.as_deref(), &bytes)
    }
}
