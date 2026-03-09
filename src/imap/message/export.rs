use std::{
    fs,
    io::{self, Write},
    num::NonZeroU32,
    path::PathBuf,
};

use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{fetch::*, select::*},
    types::fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
};
use io_stream::runtimes::std::handle;
use mail_parser::{MessageParser, MimeHeaders};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::imap::{account::ImapAccount, mailbox::arg::MailboxNameOptionalFlag, stream};

/// Export type for message export.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ExportType {
    /// Output raw RFC822 message to stdout.
    Raw,
    /// Save as .eml file.
    Eml,
    /// Export all MIME parts to separate files.
    Parts,
}

/// Export a message.
///
/// This command exports a message in various formats:
/// - raw: Output raw RFC822 message to stdout
/// - eml: Save as .eml file
/// - parts: Export all MIME parts to separate files
#[derive(Debug, Parser)]
pub struct ExportMessageCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalFlag,

    /// The message UID (or sequence number with --seq).
    #[arg(name = "id", value_name = "ID")]
    pub id: u32,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,

    /// Export type: raw (stdout), eml (file), parts (multiple files).
    #[arg(short, long, value_enum)]
    pub r#type: ExportType,

    /// Output directory (for eml and parts types).
    #[arg(short, long, value_name = "DIR")]
    pub directory: Option<PathBuf>,

    /// Open exported content in default application.
    #[arg(short = 'O', long)]
    pub open: bool,
}

impl ExportMessageCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (context, mut stream) = stream::connect(account.backend)?;

        let mailbox = self.mailbox.name.try_into()?;

        // SELECT mailbox
        let mut arg = None;
        let mut coroutine = ImapSelect::new(context, mailbox);

        let context = loop {
            match coroutine.resume(arg.take()) {
                ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapSelectResult::Ok { context, .. } => break context,
                ImapSelectResult::Err { err, .. } => bail!(err),
            }
        };

        // FETCH with BODY.PEEK[] to avoid marking as read
        let id = NonZeroU32::new(self.id).ok_or_else(|| anyhow::anyhow!("ID must be non-zero"))?;

        let item_names =
            MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::BodyExt {
                section: None,
                partial: None,
                peek: true,
            }]);

        let mut arg = None;
        let mut coroutine = ImapFetchFirst::new(context, id, item_names, !self.seq);

        let items = loop {
            match coroutine.resume(arg.take()) {
                ImapFetchFirstResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapFetchFirstResult::Ok { items, .. } => break items,
                ImapFetchFirstResult::Err { err, .. } => bail!(err),
            }
        };

        // Extract raw message bytes
        let mut raw_message: Option<Vec<u8>> = None;
        for item in items.into_iter() {
            if let MessageDataItem::BodyExt { data, .. } = item {
                if let Some(data) = data.0 {
                    raw_message = Some(data.as_ref().to_vec());
                }
            }
        }

        let raw = raw_message.ok_or_else(|| anyhow::anyhow!("No message data returned"))?;

        match self.r#type {
            ExportType::Raw => {
                // Output raw message to stdout
                io::stdout().write_all(&raw)?;
                io::stdout().flush()?;
                Ok(())
            }
            ExportType::Eml => {
                // Save as .eml file
                let message = MessageParser::default()
                    .parse(&raw)
                    .ok_or_else(|| anyhow::anyhow!("Failed to parse message"))?;

                // Generate filename from subject or message-id
                let filename = generate_eml_filename(&message, self.id);
                let dir = self.directory.unwrap_or(account.downloads_dir);

                if !dir.exists() {
                    fs::create_dir_all(&dir)?;
                }

                let path = dir.join(&filename);
                fs::write(&path, &raw)?;

                if self.open {
                    open::that(&path)?;
                }

                printer.out(Message::new(format!(
                    "Message exported to {}",
                    path.display()
                )))
            }
            ExportType::Parts => {
                // Export all MIME parts to separate files
                let message = MessageParser::default()
                    .parse(&raw)
                    .ok_or_else(|| anyhow::anyhow!("Failed to parse message"))?;

                let dir = self
                    .directory
                    .unwrap_or_else(|| PathBuf::from(format!("message_{}", self.id)));

                if !dir.exists() {
                    fs::create_dir_all(&dir)?;
                }

                let mut exported_files = Vec::new();

                for (i, part) in message.parts.iter().enumerate() {
                    // Get content type
                    let content_type = part.content_type().map(|ct| match &ct.c_subtype {
                        Some(sub) => format!("{}/{}", ct.c_type, sub),
                        None => ct.c_type.to_string(),
                    });

                    // Skip multipart container parts
                    if let Some(ref ct) = content_type {
                        if ct.starts_with("multipart/") {
                            continue;
                        }
                    }

                    let filename = generate_part_filename(part, i, &content_type);
                    let path = dir.join(&filename);

                    // Get part content
                    let contents = part.contents();
                    fs::write(&path, contents)?;
                    exported_files.push(path);
                }

                if exported_files.is_empty() {
                    bail!("No parts to export");
                }

                if self.open {
                    // Open the directory
                    open::that(&dir)?;
                }

                printer.out(Message::new(format!(
                    "Exported {} part(s) to {}",
                    exported_files.len(),
                    dir.display()
                )))
            }
        }
    }
}

fn generate_eml_filename(message: &mail_parser::Message<'_>, id: u32) -> String {
    // Try to use subject first
    if let Some(subject) = message.subject() {
        let sanitized = sanitize_filename(subject);
        if !sanitized.is_empty() {
            return format!("{}.eml", sanitized);
        }
    }

    // Fall back to message-id
    if let Some(msg_id) = message.message_id() {
        let sanitized = sanitize_filename(msg_id);
        if !sanitized.is_empty() {
            return format!("{}.eml", sanitized);
        }
    }

    // Fall back to ID
    format!("message_{}.eml", id)
}

fn generate_part_filename(
    part: &mail_parser::MessagePart<'_>,
    index: usize,
    content_type: &Option<String>,
) -> String {
    // Try to use attachment name
    if let Some(name) = part.attachment_name() {
        return sanitize_filename(name);
    }

    // Generate filename based on content type
    let ext = content_type
        .as_ref()
        .and_then(|ct| extension_from_content_type(ct))
        .unwrap_or("bin");

    format!("part_{}.{}", index, ext)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .chars()
        .take(200) // Limit filename length
        .collect()
}

fn extension_from_content_type(content_type: &str) -> Option<&'static str> {
    match content_type {
        "text/plain" => Some("txt"),
        "text/html" => Some("html"),
        "text/css" => Some("css"),
        "text/javascript" | "application/javascript" => Some("js"),
        "text/xml" | "application/xml" => Some("xml"),
        "text/csv" => Some("csv"),
        "text/calendar" => Some("ics"),
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        "image/bmp" => Some("bmp"),
        "image/tiff" => Some("tiff"),
        "audio/mpeg" => Some("mp3"),
        "audio/ogg" => Some("ogg"),
        "audio/wav" => Some("wav"),
        "video/mp4" => Some("mp4"),
        "video/webm" => Some("webm"),
        "video/mpeg" => Some("mpeg"),
        "application/pdf" => Some("pdf"),
        "application/zip" => Some("zip"),
        "application/gzip" => Some("gz"),
        "application/x-tar" => Some("tar"),
        "application/json" => Some("json"),
        "application/octet-stream" => Some("bin"),
        "message/rfc822" => Some("eml"),
        _ => None,
    }
}
