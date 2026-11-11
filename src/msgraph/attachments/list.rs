//! # Microsoft Graph attachment list
//!
//! The `msgraph attachments list` command, listing the attachments of a
//! message.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::rest::users::messages::attachments::MsgraphAttachment;
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, msgraph::client::MsgraphClient};

/// List a message's attachments (`GET /me/messages/{id}/attachments`).
#[derive(Debug, Parser)]
pub struct MsgraphAttachmentListCommand {
    /// The id of the message whose attachments to list.
    #[arg(value_name = "MESSAGE_ID")]
    pub message_id: String,
}

impl MsgraphAttachmentListCommand {
    /// Lists the attachments of the message and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let attachments = client.attachments_list(&self.message_id)?.response.value;

        printer.out(MsgraphAttachmentListOutput { attachments })
    }
}

/// The `msgraph attachments list` output, the attachments of a message.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MsgraphAttachmentListOutput {
    attachments: Vec<MsgraphAttachment>,
}

impl fmt::Display for MsgraphAttachmentListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: a name is free text, so it reads last.
        let mut columns = Columns::new(["ID", "TYPE", "SIZE", "INLINE", "NAME"]);

        for attachment in &self.attachments {
            let name = attachment.name.as_deref().unwrap_or_default();
            let content_type = attachment.content_type.as_deref().unwrap_or_default();
            let size = attachment.size.map(|n| n.to_string()).unwrap_or_default();
            let inline = if attachment.is_inline == Some(true) {
                "yes"
            } else {
                ""
            };

            columns.push([
                sanitize::text(&attachment.id).into_owned(),
                sanitize::text(content_type).into_owned(),
                size,
                String::from(inline),
                sanitize::text(name).into_owned(),
            ]);
        }

        write!(f, "{columns}")
    }
}
