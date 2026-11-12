//! # Microsoft Graph attachment list
//!
//! The `msgraph attachments list` command, listing the attachments of a
//! message.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use humansize::{BINARY, format_size};
use io_msgraph::v1::rest::users::messages::attachments::MsgraphAttachment;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, backend::Backend, listing::Listing, msgraph::client::MsgraphClient,
};

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
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let attachments = client.attachments_list(&self.message_id)?.response.value;

        printer.out(MsgraphAttachmentListOutput {
            account: account.name().to_string(),
            message: self.message_id,
            attachments,
        })
    }
}

/// The `msgraph attachments list` output, the attachments of a message.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MsgraphAttachmentListOutput {
    #[serde(skip)]
    account: String,
    #[serde(skip)]
    message: String,
    attachments: Vec<MsgraphAttachment>,
}

impl fmt::Display for MsgraphAttachmentListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Msgraph));
        listing.head("Message", &self.message);

        for attachment in &self.attachments {
            let inline = match attachment.is_inline {
                Some(true) => "inline",
                _ => "",
            };

            let size = attachment.size.map(|n| n.max(0) as u64);
            let size = size.map(|n| format_size(n, BINARY.space_after_value(false)));
            let content_type = attachment.content_type.as_deref().unwrap_or_default();
            let name = attachment.name.as_deref().unwrap_or_default();

            listing.items.push([
                sanitize::text(&attachment.id).into_owned(),
                String::from(inline),
                size.unwrap_or_default(),
                sanitize::text(content_type).into_owned(),
                sanitize::text(name).into_owned(),
            ]);
        }

        write!(f, "{listing}")
    }
}
