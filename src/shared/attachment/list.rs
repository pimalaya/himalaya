//! # Attachment list
//!
//! The `attachment list` command, tabling the attachment parts of one
//! message.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use humansize::{BINARY, format_size};
use mail_parser::{Message, MessagePart, MimeHeaders};
use pimalaya_cli::printer::Printer;
use pimalaya_cli::table::{Cell, Color, ContentArrangement, Row, Table, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    shared::{
        client::EmailClient, mailbox::arg::MailboxArg, message::part, table::style_from_preset,
    },
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
        let mailbox = self.mailbox.resolve(account);
        let raw = client.get_message(&mailbox, &self.message_id, false)?;

        let message = part::parse(&raw)?;

        let attachments = attachments(&message)
            .filter(|attachment| self.inline || !attachment.inline)
            .collect();

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

/// The attachment parts of a message, in part order, inline ones
/// included: mail-parser's attachments, under the ids of the shared part
/// walk.
pub(crate) fn attachments<'a>(message: &'a Message) -> impl Iterator<Item = Attachment> + 'a {
    part::leaves(message)
        .filter(|leaf| message.attachments.contains(&(leaf.index as u32)))
        .map(|leaf| Attachment::new(message, leaf, None))
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
    /// The `Content-ID`, without angle brackets, by which an HTML body
    /// shows the part.
    #[serde(rename = "contentId")]
    pub content_id: Option<String>,
    /// Where the bytes were written, which `attachment download` alone
    /// fills in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Attachment {
    /// The row of one leaf part, `path` being where it was written.
    pub(crate) fn new(message: &Message, leaf: part::Leaf, path: Option<String>) -> Self {
        let part = leaf.part;
        Self {
            id: leaf.id,
            filename: part::filename(part),
            mime: mime_string(part),
            size: part::bytes(message, part).len() as u64,
            inline: part
                .content_disposition()
                .is_some_and(|cd| cd.c_type.eq_ignore_ascii_case("inline")),
            content_id: part::content_id(part),
            path,
        }
    }
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
                row.add_cell(Cell::new(sanitize(&a.id)).fg(self.colors.id));
                row.add_cell(
                    Cell::new(sanitize(a.filename.as_deref().unwrap_or("")))
                        .fg(self.colors.filename),
                );
                row.add_cell(
                    Cell::new(sanitize(a.mime.as_deref().unwrap_or(""))).fg(self.colors.r#type),
                );
                row.add_cell(Cell::new(format_size(a.size, BINARY)).fg(self.colors.size));
                if self.with_inline {
                    row.add_cell(
                        Cell::new(if a.inline { "yes" } else { "no" }).fg(self.colors.inline),
                    );
                }
                if self.with_path {
                    row.add_cell(
                        Cell::new(sanitize(a.path.as_deref().unwrap_or(""))).fg(self.colors.path),
                    );
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

#[cfg(test)]
mod tests {
    use pimalaya_cli::table::{Color, ContentArrangement};

    use super::{Attachment, AttachmentColors, Attachments, attachments};
    use crate::shared::message::{part, read::MessageReadOutput};

    #[test]
    fn ids_are_those_of_the_read_view() {
        let raw = b"Content-Type: multipart/mixed; boundary=\"m\"\r\n\
            \r\n\
            --m\r\n\
            Content-Type: multipart/related; boundary=\"r\"\r\n\
            \r\n\
            --r\r\n\
            Content-Type: text/html\r\n\
            \r\n\
            <img src=\"cid:logo@example.com\">\r\n\
            --r\r\n\
            Content-Type: image/png\r\n\
            Content-ID: <logo@example.com>\r\n\
            Content-Disposition: inline\r\n\
            \r\n\
            PNG\r\n\
            --r--\r\n\
            --m\r\n\
            Content-Type: application/pdf; name=\"doc.pdf\"\r\n\
            Content-Transfer-Encoding: base64\r\n\
            \r\n\
            JVBERi0=\r\n\
            --m\r\n\
            Content-Type: message/rfc822\r\n\
            \r\n\
            Subject: inner\r\n\
            \r\n\
            inner\r\n\
            --m--\r\n";
        let message = part::parse(raw).unwrap();
        let listed: Vec<Attachment> = attachments(&message).collect();
        let view = serde_json::to_value(MessageReadOutput::new(part::parse(raw).unwrap())).unwrap();
        let parts = view["parts"].as_array().unwrap();

        let ids: Vec<&str> = listed.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["4", "5", "6"]);
        for attachment in &listed {
            let part = parts
                .iter()
                .find(|part| part["id"] == attachment.id.as_str())
                .expect("listed id in the view");
            assert_eq!(part["size"], attachment.size);
            assert_eq!(part["contentId"], serde_json::json!(attachment.content_id));
            assert_ne!(part["role"], "body");
        }

        let json = serde_json::to_value(&listed[0]).unwrap();
        assert_eq!(json["contentId"], "logo@example.com");
        assert_eq!(json["inline"], true);
        assert_eq!(
            serde_json::to_value(&listed[1]).unwrap()["contentId"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn control_characters_from_the_sender_are_not_printed() {
        let attachments = Attachments {
            preset: String::new(),
            arrangement: ContentArrangement::Disabled,
            with_inline: false,
            with_path: true,
            colors: AttachmentColors {
                id: Color::Reset,
                filename: Color::Reset,
                r#type: Color::Reset,
                size: Color::Reset,
                inline: Color::Reset,
                path: Color::Reset,
            },
            attachments: vec![Attachment {
                id: String::from("1"),
                filename: Some(String::from("invoice\x1b]8;;https://example.org\x07.pdf")),
                mime: Some(String::from("application/pdf\x1b[8m")),
                size: 0,
                inline: false,
                content_id: None,
                path: Some(String::from("/tmp/invoice\x1b[2J.pdf")),
            }],
        };

        let output = attachments.to_string();

        assert!(!output.contains('\x1b'), "{output:?}");
        assert!(!output.contains('\x07'), "{output:?}");
        assert!(output.contains("invoice"), "{output:?}");
    }
}
