//! # Attachment list
//!
//! The `attachment list` command, tabling the attachment parts of one
//! message.

use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use humansize::{BINARY, format_size};
use mail_parser::{MessageParser, MessagePart, MimeHeaders};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    shared::{client::EmailClient, mailbox::arg::MailboxArg, table::style_from_preset},
};

/// List the attachments of one message.
///
/// The ID of a row is the 1-based position of the MIME part in the whole
/// message, the same id `message read` prints. Only attachment parts are
/// listed, so the ids are sparse, and they stay the same whether or not
/// `--inline` is passed.
#[derive(Debug, Parser)]
pub struct AttachmentListCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Identifier of the message.
    #[arg(value_name = "MESSAGE-ID")]
    pub message_id: String,
    /// Also list the parts carrying `Content-Disposition: inline`,
    /// typically the images an HTML body references through `cid:`.
    #[arg(long, short)]
    pub inline: bool,
}

impl AttachmentListCommand {
    /// Fetches the message and prints its attachment parts as a table.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account)?;
        let raw = client.get_message(&mailbox, &self.message_id, false)?;

        let Some(message) = MessageParser::new().parse(&raw) else {
            bail!("Failed to parse RFC 5322 message");
        };

        let mut attachments = Vec::new();
        for &part_id in &message.attachments {
            let part = &message.parts[part_id as usize];
            let inline = part
                .content_disposition()
                .map(|cd| cd.c_type.eq_ignore_ascii_case("inline"))
                .unwrap_or(false);

            if inline && !self.inline {
                continue;
            }

            attachments.push(Attachment {
                id: (part_id + 1).to_string(),
                filename: part.attachment_name().map(str::to_owned),
                mime: mime_string(part),
                size: part.contents().len() as u64,
                inline,
                path: None,
            });
        }

        let attachments = Attachments {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            with_inline: self.inline,
            with_path: false,
            colors: AttachmentColors {
                id: account.attachments_list_table_id_color(),
                filename: account.attachments_list_table_filename_color(),
                r#type: account.attachments_list_table_type_color(),
                size: account.attachments_list_table_size_color(),
                inline: account.attachments_list_table_inline_color(),
                path: account.attachments_list_table_path_color(),
            },
            attachments,
        };

        printer.out(attachments)
    }
}

/// Per-column colors of the attachments table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AttachmentColors {
    pub id: Color,
    pub filename: Color,
    pub r#type: Color,
    pub size: Color,
    pub inline: Color,
    pub path: Color,
}

/// One row of the `attachment list` and `attachment download` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Attachment {
    /// The 1-based position of the MIME part in the message, the same id
    /// `message read` prints.
    pub id: String,
    /// The RFC 2231-decoded filename, `None` when the part names none.
    pub filename: Option<String>,
    /// The MIME type, `None` when the part carries no `Content-Type`.
    pub mime: Option<String>,
    /// The size of the decoded part body, in bytes.
    pub size: u64,
    /// Whether the part carries `Content-Disposition: inline`.
    pub inline: bool,
    /// Where the bytes were written, which `attachment download` alone
    /// fills in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// The `attachment list` output, a table of attachments.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Attachments {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The column arrangement the table renders with.
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    /// Whether the INLINE column is drawn.
    #[serde(skip)]
    pub with_inline: bool,
    /// Whether the PATH column is drawn.
    #[serde(skip)]
    pub with_path: bool,
    #[serde(skip)]
    pub(crate) colors: AttachmentColors,
    /// The attachments, in part order.
    pub attachments: Vec<Attachment>,
}

impl fmt::Display for Attachments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        let mut header = vec![
            Cell::new("ID"),
            Cell::new("FILENAME"),
            Cell::new("TYPE"),
            Cell::new("SIZE"),
        ];
        if self.with_inline {
            header.push(Cell::new("INLINE"));
        }
        if self.with_path {
            header.push(Cell::new("PATH"));
        }

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from(header))
            .add_rows(self.attachments.iter().map(|a| {
                let mut row = Row::new();
                row.max_height(1);
                row.add_cell(Cell::new(&a.id).fg(self.colors.id));
                row.add_cell(
                    Cell::new(a.filename.as_deref().unwrap_or("")).fg(self.colors.filename),
                );
                row.add_cell(Cell::new(a.mime.as_deref().unwrap_or("")).fg(self.colors.r#type));
                row.add_cell(Cell::new(format_size(a.size, BINARY)).fg(self.colors.size));
                if self.with_inline {
                    row.add_cell(
                        Cell::new(if a.inline { "yes" } else { "no" }).fg(self.colors.inline),
                    );
                }
                if self.with_path {
                    row.add_cell(Cell::new(a.path.as_deref().unwrap_or("")).fg(self.colors.path));
                }
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

/// Renders a part's `Content-Type` as `type/subtype`.
pub(super) fn mime_string(part: &MessagePart<'_>) -> Option<String> {
    let ct = part.content_type()?;

    Some(match ct.c_subtype.as_deref() {
        Some(sub) => format!("{}/{}", ct.c_type, sub),
        None => ct.c_type.to_string(),
    })
}
