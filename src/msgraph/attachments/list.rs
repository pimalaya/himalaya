//! # Microsoft Graph attachment list
//!
//! The `msgraph attachments list` command, listing the attachments of a
//! message.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use humansize::{BINARY, format_size};
use io_msgraph::v1::{rest::users::messages::attachments::MsgraphAttachment, send::path_segment};
use pimalaya_cli::{listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    account::context::Account,
    backend::Backend,
    msgraph::{client::MsgraphClient, messages::list::raw_page},
};

/// List a message's attachments (`GET /me/messages/{id}/attachments`).
///
/// JSON output: `{"attachments": [<raw Graph attachment>...]}`, every
/// page Graph links to read.
#[derive(Debug, Parser)]
pub struct MsgraphAttachmentListCommand {
    /// The id of the message whose attachments to list.
    #[arg(value_name = "MESSAGE_ID")]
    pub message_id: String,
}

impl MsgraphAttachmentListCommand {
    /// Lists the attachments of the message, every page Graph links to
    /// read, and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let path = format!("messages/{}/attachments", path_segment(&self.message_id)?);
        let mut page = raw_page(client, &path, &[])?;
        let mut attachments = Vec::new();

        loop {
            attachments.append(&mut page.value);

            let Some(next) = page.next_link else {
                break;
            };
            page = client.page_get(&next, None)?.response;
        }

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
    /// The attachments of every page, each as Graph sent it, its
    /// `contentBytes` included.
    #[schemars(with = "Vec<Map<String, Value>>")]
    attachments: Vec<Value>,
}

impl fmt::Display for MsgraphAttachmentListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Msgraph.as_str()));
        listing.head("Message", &self.message);

        for attachment in &self.attachments {
            let attachment: MsgraphAttachment =
                serde_json::from_value(attachment.clone()).unwrap_or_default();
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
