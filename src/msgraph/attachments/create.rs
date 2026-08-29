//! # Microsoft Graph attachment create
//!
//! The `msgraph attachments create` command, posting a file attachment
//! onto a message.

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::msgraph::client::MsgraphClient;

/// Add a file attachment to a message.
///
/// The file is uploaded as a `fileAttachment` under its own name, and the
/// content type is left for Graph to infer unless `--content-type` names
/// one.
#[derive(Debug, Parser)]
pub struct MsgraphAttachmentCreateCommand {
    /// The id of the message to attach the file to.
    #[arg(value_name = "MESSAGE_ID")]
    pub message_id: String,
    /// The path to the file to upload as an attachment.
    #[arg(value_name = "PATH")]
    pub path: PathBuf,
    /// Override the attachment name (defaults to the file name).
    #[arg(short = 'n', long, value_name = "NAME")]
    pub name: Option<String>,
    /// Set the attachment content type (e.g. `application/pdf`).
    #[arg(short = 't', long, value_name = "TYPE")]
    pub content_type: Option<String>,
}

impl MsgraphAttachmentCreateCommand {
    /// Posts the file onto the message as an attachment.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let content = std::fs::read(&self.path)?;

        let name = match self.name {
            Some(name) => name,
            None => self
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    anyhow!(
                        "Cannot derive attachment name from `{}`",
                        self.path.display()
                    )
                })?
                .to_owned(),
        };

        let attachment = client
            .attachment_create(
                &self.message_id,
                &name,
                &content,
                self.content_type.as_deref(),
            )?
            .response;

        printer.out(Message::new(format!(
            "Microsoft Graph attachment `{}` successfully created",
            attachment.id
        )))
    }
}
