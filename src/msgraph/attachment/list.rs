use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_msgraph::v1::rest::users::messages::attachments::MsgraphAttachment;
use pimalaya_cli::printer::Printer;
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
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let attachments = client.attachments_list(&self.message_id)?.response.value;

        let table = AttachmentsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            colors: AttachmentColors {
                id: account.attachments_list_table_id_color(),
                name: account.attachments_list_table_filename_color(),
                content_type: account.attachments_list_table_type_color(),
                size: account.attachments_list_table_size_color(),
                inline: account.attachments_list_table_inline_color(),
            },
            attachments,
        };

        printer.out(table)
    }
}

/// Per-column colors for the Microsoft Graph attachments table.
#[derive(Clone, Copy, Debug)]
pub struct AttachmentColors {
    pub id: Color,
    pub name: Color,
    pub content_type: Color,
    pub size: Color,
    pub inline: Color,
}

impl Default for AttachmentColors {
    fn default() -> Self {
        Self {
            id: Color::Reset,
            name: Color::Reset,
            content_type: Color::Reset,
            size: Color::Reset,
            inline: Color::Reset,
        }
    }
}

/// Renderable table of Microsoft Graph message attachments.
#[derive(Clone, Debug, Serialize)]
pub struct AttachmentsTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    #[serde(skip)]
    colors: AttachmentColors,
    attachments: Vec<MsgraphAttachment>,
}

impl fmt::Display for AttachmentsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("NAME"),
                Cell::new("TYPE"),
                Cell::new("SIZE"),
                Cell::new("INLINE"),
            ]))
            .add_rows(self.attachments.iter().map(|attachment| {
                let name = attachment.name.clone().unwrap_or_default();
                let content_type = attachment.content_type.clone().unwrap_or_default();
                let size = attachment.size.map(|n| n.to_string()).unwrap_or_default();
                let inline = if attachment.is_inline == Some(true) {
                    "yes"
                } else {
                    ""
                };

                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&attachment.id).fg(self.colors.id))
                    .add_cell(Cell::new(name).fg(self.colors.name))
                    .add_cell(Cell::new(content_type).fg(self.colors.content_type))
                    .add_cell(Cell::new(size).fg(self.colors.size))
                    .add_cell(Cell::new(inline).fg(self.colors.inline));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
