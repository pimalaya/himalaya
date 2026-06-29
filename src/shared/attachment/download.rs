use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};
use clap::Parser;
use mail_parser::{MessageParser, MimeHeaders};
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::shared::{
    attachment::list::{Attachment, AttachmentColors, Attachments, mime_string},
    client::EmailClient,
    mailbox::arg::MailboxArg,
};

/// Download specific attachments of a single message to disk.
///
/// The attachment ids are the 1-based positions reported by
/// `attachments list`. Pass one or more ids to fetch exactly those
/// parts. Inline parts are addressable by their id too — the id you
/// see in `attachments list --inline` is the same id you pass here.
///
/// The destination directory defaults to the account's
/// `downloads-dir` config (falling back to the global one, then the
/// platform's standard downloads directory). Pass `--dir <PATH>` to
/// override.
#[derive(Debug, Parser)]
pub struct AttachmentDownloadCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,

    /// Identifier of the message.
    #[arg(value_name = "MESSAGE-ID")]
    pub message_id: String,

    /// Attachment identifier(s) to download.
    ///
    /// Omit identifiers to download all attachments.
    #[arg(value_name = "ATTACHMENT-ID", num_args = 0..)]
    pub attachment_ids: Vec<String>,

    /// Destination directory.
    ///
    /// Overrides the account/global `downloads-dir` config.
    #[arg(long, short, value_name = "PATH")]
    pub dir: Option<PathBuf>,
}

impl AttachmentDownloadCommand {
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

        let dir = self.dir.clone().unwrap_or_else(|| account.downloads_dir());

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        let wanted_all = self.attachment_ids.is_empty();
        let mut remaining: BTreeSet<String> = self.attachment_ids.iter().cloned().collect();
        let mut written = Vec::new();

        for (index, part) in message.attachments().enumerate() {
            let id = (index + 1).to_string();
            if !wanted_all && !remaining.remove(&id) {
                continue;
            }

            let inline = part
                .content_disposition()
                .map(|cd| cd.c_type.eq_ignore_ascii_case("inline"))
                .unwrap_or(false);
            let filename = part.attachment_name().map(str::to_owned);
            let on_disk_name = filename
                .clone()
                .unwrap_or_else(|| format!("attachment-{id}"));
            let safe = sanitize(&on_disk_name);
            let path = unique_path(&dir, &safe);

            fs::write(&path, part.contents())?;

            written.push(Attachment {
                id,
                filename,
                mime: mime_string(part),
                size: part.contents().len() as u64,
                inline,
                path: Some(path.display().to_string()),
            });
        }

        if !remaining.is_empty() {
            let missing: Vec<String> = remaining.into_iter().collect();
            bail!(
                "No attachment with id {} on message `{}`",
                missing.join(", "),
                self.message_id,
            );
        }

        let attachments = Attachments {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            with_inline: written.iter().any(|a| a.inline),
            with_path: true,
            colors: AttachmentColors {
                id: account.attachments_list_table_id_color(),
                filename: account.attachments_list_table_filename_color(),
                r#type: account.attachments_list_table_type_color(),
                size: account.attachments_list_table_size_color(),
                inline: account.attachments_list_table_inline_color(),
                path: account.attachments_list_table_path_color(),
            },
            attachments: written,
        };

        printer.out(attachments)
    }
}

/// Strips path separators and parent traversals so a hostile filename
/// header can't escape the download directory.
fn sanitize(name: &str) -> String {
    let trimmed = name.trim();
    let cleaned: String = trimmed
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' => '_',
            _ => c,
        })
        .collect();
    let cleaned = cleaned.trim_start_matches('.').trim();
    if cleaned.is_empty() {
        "attachment".to_string()
    } else {
        cleaned.to_string()
    }
}

/// Returns a path inside `dir` that doesn't already exist by suffixing
/// `(1)`, `(2)`, … to the stem when needed.
fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }

    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };

    for n in 1..1024 {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(name)
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn keeps_a_plain_filename() {
        assert_eq!(sanitize("report.pdf"), "report.pdf");
    }

    #[test]
    fn replaces_path_separators() {
        assert_eq!(sanitize("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize("a/b\\c"), "a_b_c");
    }

    #[test]
    fn collapses_traversal_and_dot_names_to_the_fallback() {
        assert_eq!(sanitize(".."), "attachment");
        assert_eq!(sanitize("."), "attachment");
        assert_eq!(sanitize(""), "attachment");
        assert_eq!(sanitize("   "), "attachment");
    }

    #[test]
    fn strips_leading_dots() {
        assert_eq!(sanitize(".hidden"), "hidden");
    }
}
