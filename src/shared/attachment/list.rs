//! # Attachment list
//!
//! The `attachment list` command, listing the attachment parts of one
//! message.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use humansize::{BINARY, format_size};
use mime::tree::{cst::MimeCst, entity::MimeEntity};
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    shared::{
        client::EmailClient,
        mailbox::arg::MailboxArg,
        message::part::{self, PartRole},
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
    /// Fetches the message and prints its attachment parts.
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
            .filter(|attachment| self.inline || attachment.role != PartRole::Inline)
            .collect();

        let attachments = AttachmentListOutput {
            with_inline: self.inline,
            with_path: false,
            attachments,
        };

        printer.out(attachments)
    }
}

/// The attachment parts of a message, in part order, inline ones
/// included: the attachment list of RFC 8621 §4.1.4 mime-rs computes,
/// under the ids of the shared part walk.
pub(crate) fn attachments<'a>(message: &'a MimeCst) -> impl Iterator<Item = Attachment> + 'a {
    let listed = message.root.body_lists().attachments;

    part::leaves(message)
        .filter(move |leaf| listed.iter().any(|part| std::ptr::eq(*part, leaf.part)))
        .map(|leaf| Attachment::new(leaf, None))
}

/// One row of the `attachment list` and `attachment download` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
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
    /// What the part is to a reader, the role `message read` prints.
    pub role: PartRole,
    /// The `Content-ID`, without angle brackets, by which an HTML body
    /// shows the part.
    #[serde(rename = "contentId")]
    pub content_id: Option<String>,
    /// Where the bytes were written, which `attachment download` alone
    /// fills in, `null` otherwise.
    pub path: Option<String>,
}

impl Attachment {
    /// The row of one leaf part, `path` being where it was written.
    pub(crate) fn new(leaf: part::Leaf, path: Option<String>) -> Self {
        let part = leaf.part;
        Self {
            id: leaf.id,
            filename: part::filename(part),
            mime: mime_string(part),
            size: part::bytes(part).len() as u64,
            role: part::role(part),
            content_id: part::content_id(part),
            path,
        }
    }
}

/// The `attachment list` output, the attachments of one message.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentListOutput {
    /// Whether the INLINE column is drawn.
    #[serde(skip)]
    pub with_inline: bool,
    /// Whether the PATH column is drawn.
    #[serde(skip)]
    pub with_path: bool,
    /// The attachments, in part order.
    pub attachments: Vec<Attachment>,
}

impl fmt::Display for AttachmentListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: a file name is free text, so it reads last, but for the
        // path a download wrote it to, which ends with it.
        let mut header = vec!["ID", "TYPE", "SIZE"];
        if self.with_inline {
            header.push("INLINE");
        }
        header.push("FILENAME");
        if self.with_path {
            header.push("PATH");
        }

        let mut columns = Columns::new(header);

        for attachment in &self.attachments {
            let text = |value: &Option<String>| {
                sanitize::text(value.as_deref().unwrap_or_default()).into_owned()
            };

            let mut row = vec![
                sanitize::text(&attachment.id).into_owned(),
                text(&attachment.mime),
                format_size(attachment.size, BINARY),
            ];
            if self.with_inline {
                let inline = attachment.role == PartRole::Inline;
                row.push(String::from(if inline { "yes" } else { "no" }));
            }
            row.push(text(&attachment.filename));
            if self.with_path {
                row.push(text(&attachment.path));
            }

            columns.push(row);
        }

        write!(f, "{columns}")
    }
}

/// Renders a part's `Content-Type` as `type/subtype`.
pub(super) fn mime_string(part: &MimeEntity) -> Option<String> {
    part.header.get("Content-Type")?;
    let ctype = part.content_type();
    let top = String::from_utf8_lossy(ctype.top);

    Some(match ctype.sub {
        [] => top.into_owned(),
        sub => format!("{top}/{}", String::from_utf8_lossy(sub)),
    })
}

#[cfg(test)]
mod tests {
    use crate::shared::{
        attachment::list::{Attachment, AttachmentListOutput, attachments},
        message::{
            part::{self, PartRole},
            read::MessageReadOutput,
        },
    };

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
        assert_eq!(json["role"], "inline");
        assert_eq!(
            serde_json::to_value(&listed[1]).unwrap()["contentId"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn control_characters_from_the_sender_are_not_printed() {
        let attachments = AttachmentListOutput {
            with_inline: false,
            with_path: true,
            attachments: vec![Attachment {
                id: String::from("1"),
                filename: Some(String::from("invoice\x1b]8;;https://example.org\x07.pdf")),
                mime: Some(String::from("application/pdf\x1b[8m")),
                size: 0,
                role: PartRole::Attachment,
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
