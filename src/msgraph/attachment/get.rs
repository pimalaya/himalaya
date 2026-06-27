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
    #[arg(value_name = "MESSAGE_ID")]
    pub message_id: String,

    #[arg(value_name = "ATTACHMENT_ID")]
    pub id: String,

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
