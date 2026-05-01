use anyhow::{bail, Result};
use clap::Parser;
use mail_parser::{MessageParser, MessagePart, MimeHeaders};
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    account::Account,
    attachments::table::{AttachmentEntry, AttachmentsTable},
    cli::BackendArg,
    config::{AccountConfig, Config},
};

/// List the attachments carried by a single message in the active
/// account.
///
/// "Attachment" follows mail_parser's classification: parts with
/// `Content-Disposition: attachment`, or any non-body part with a
/// `filename`/`name` parameter. Inline parts (e.g. embedded images
/// referenced by HTML bodies) are skipped by default; pass
/// `--include-inline` to surface them too.
#[derive(Debug, Parser)]
pub struct AttachmentsListCommand {
    /// Identifier of the message (IMAP UID, JMAP email id, or Maildir
    /// filename id).
    #[arg(value_name = "ID")]
    pub id: String,

    /// Mailbox name or path (IMAP/Maildir). Ignored for JMAP.
    #[arg(
        long = "mailbox",
        short = 'm',
        value_name = "NAME",
        default_value = "Inbox"
    )]
    pub mailbox: String,

    /// Include parts with `Content-Disposition: inline`.
    #[arg(long = "include-inline")]
    pub include_inline: bool,
}

impl AttachmentsListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        let raw = crate::messages::fetch::fetch_raw(
            &config,
            &account_config,
            backend,
            &self.mailbox,
            &self.id,
        )?;

        let Some(message) = MessageParser::new().parse(&raw) else {
            bail!("Failed to parse RFC 5322 message");
        };

        let mut attachments = Vec::new();
        for (index, part) in message.attachments().enumerate() {
            let inline = is_inline(part);
            if inline && !self.include_inline {
                continue;
            }

            attachments.push(AttachmentEntry {
                index,
                filename: part
                    .attachment_name()
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("attachment-{index}")),
                mime: mime_string(part),
                size: part.contents().len(),
                inline,
            });
        }

        // Reuse the active account's table styling. Constructing
        // an `Account<()>` is enough to read the preset/arrangement.
        let account = Account::new(config, account_config, ())?;

        printer.out(AttachmentsTable {
            preset: account.table_preset,
            arrangement: account.table_arrangement,
            attachments,
        })
    }
}

fn is_inline(part: &MessagePart<'_>) -> bool {
    part.content_disposition()
        .map(|cd| cd.c_type.eq_ignore_ascii_case("inline"))
        .unwrap_or(false)
}

fn mime_string(part: &MessagePart<'_>) -> String {
    let Some(ct) = part.content_type() else {
        return "application/octet-stream".to_string();
    };
    match ct.c_subtype.as_deref() {
        Some(sub) => format!("{}/{}", ct.c_type, sub),
        None => ct.c_type.to_string(),
    }
}
