use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use humansize::{BINARY, format_size};
use mail_parser::{MessageParser, MessagePart, MimeHeaders};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::shared::{client::EmailClient, mailbox::arg::MailboxArg};

/// List the attachments carried by a single message in the active
/// account.
///
/// Each row carries a 1-based `ID` matching the position of the part
/// in mail_parser's attachment iteration order. The `ID` is stable
/// regardless of the `--inline` filter — listing only the attachment
/// parts and listing every non-body part assign the same id to the
/// same underlying part. So if a message has parts `1=attachment,
/// 2=attachment, 3=inline, 4=attachment`, the default listing shows
/// `1 2 4` and `--inline` shows `1 2 3 4`.
///
/// Pass `--inline` to surface inline parts (typically embedded images
/// referenced by HTML bodies via `cid:`).
#[derive(Debug, Parser)]
pub struct AttachmentListCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Identifier of the message.
    #[arg(value_name = "MESSAGE-ID")]
    pub message_id: String,
    /// Include parts with `Content-Disposition: inline`.
    #[arg(long, short)]
    pub inline: bool,
}

impl AttachmentListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account)?;
        let raw = client.get_message(&mailbox, &self.message_id)?;

        let Some(message) = MessageParser::new().parse(&raw) else {
            bail!("Failed to parse RFC 5322 message");
        };

        let mut attachments = Vec::new();
        for (index, part) in message.attachments().enumerate() {
            let inline = part
                .content_disposition()
                .map(|cd| cd.c_type.eq_ignore_ascii_case("inline"))
                .unwrap_or(false);

            if inline && !self.inline {
                continue;
            }

            attachments.push(Attachment {
                id: (index + 1).to_string(),
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

#[derive(Clone, Copy, Debug)]
pub(crate) struct AttachmentColors {
    pub id: Color,
    pub filename: Color,
    pub r#type: Color,
    pub size: Color,
    pub inline: Color,
    pub path: Color,
}

/// One row of the `attachments list` / `attachments download` output.
#[derive(Clone, Debug, Serialize)]
pub struct Attachment {
    /// 1-based linear index in mail-parser's attachment iteration
    /// order. Stable across the `--inline` filter.
    pub id: String,
    /// Filename from `Content-Disposition: filename=` (or
    /// `Content-Type: name=`), RFC 2231-decoded. `None` when the
    /// source provides no name.
    pub filename: Option<String>,
    /// MIME type (e.g. `"application/pdf"`). `None` when the source
    /// omits the `Content-Type` header.
    pub mime: Option<String>,
    /// Size in bytes of the decoded part body.
    pub size: u64,
    /// `true` when the part carries `Content-Disposition: inline`.
    pub inline: bool,
    /// Destination path the bytes were written to (set by
    /// `attachments download`; `None` for `attachments list`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Table of attachment rows rendered to the terminal or as JSON.
#[derive(Clone, Debug, Serialize)]
pub struct Attachments {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    #[serde(skip)]
    pub with_inline: bool,
    #[serde(skip)]
    pub with_path: bool,
    #[serde(skip)]
    pub(crate) colors: AttachmentColors,
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
            .load_preset(&self.preset)
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

pub(super) fn mime_string(part: &MessagePart<'_>) -> Option<String> {
    let ct = part.content_type()?;

    Some(match ct.c_subtype.as_deref() {
        Some(sub) => format!("{}/{}", ct.c_type, sub),
        None => ct.c_type.to_string(),
    })
}
