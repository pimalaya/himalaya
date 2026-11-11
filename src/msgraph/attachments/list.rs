//! # Microsoft Graph attachment list
//!
//! The `msgraph attachments list` command, listing the attachments of a
//! message.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::rest::users::messages::attachments::MsgraphAttachment;
use pimalaya_cli::{
    fields::Fields,
    printer::Printer,
    records::{Record, Records},
    sanitize,
};
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
        let mut records = Records::default();

        for attachment in &self.attachments {
            let name = attachment.name.as_deref().unwrap_or_default();
            let content_type = attachment.content_type.as_deref().unwrap_or_default();
            let size = attachment.size.map(|n| n.to_string()).unwrap_or_default();

            let mut state = Vec::new();
            if attachment.is_inline == Some(true) {
                state.push(String::from("inline"));
            }

            let mut fields = Fields::default();
            fields.push("Name", sanitize::text(name));
            fields.push("Type", sanitize::text(content_type));
            fields.push("Size", size);

            records.push(Record {
                title: sanitize::text(&attachment.id).into_owned(),
                state,
                fields,
                body: String::new(),
            });
        }

        write!(f, "{records}")
    }
}
