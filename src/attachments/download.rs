use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use clap::Parser;
use mail_parser::{MessageParser, MimeHeaders};
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    account::Account,
    cli::BackendArg,
    config::{AccountConfig, Config},
};

/// Download the attachments carried by a single message to disk.
///
/// "Attachment" follows mail_parser's classification: parts with
/// `Content-Disposition: attachment`, or any non-body part with a
/// `filename`/`name` parameter. Inline parts are skipped by default;
/// pass `--include-inline` to download them too.
///
/// The destination directory defaults to the account's
/// `downloads-dir` config (falling back to the global one, then the
/// platform's standard downloads directory). Pass `--dir <PATH>` to
/// override.
#[derive(Debug, Parser)]
pub struct AttachmentsDownloadCommand {
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

    /// Destination directory. Overrides the account/global
    /// `downloads-dir` config.
    #[arg(long = "dir", short = 'd', value_name = "PATH")]
    pub dir: Option<PathBuf>,

    /// Include parts with `Content-Disposition: inline`.
    #[arg(long = "include-inline")]
    pub include_inline: bool,
}

impl AttachmentsDownloadCommand {
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

        let account = Account::new(config, account_config, ())?;
        let dir = self.dir.clone().unwrap_or(account.downloads_dir);

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        let mut written = Vec::new();
        for (index, part) in message.attachments().enumerate() {
            let inline = part
                .content_disposition()
                .map(|cd| cd.c_type.eq_ignore_ascii_case("inline"))
                .unwrap_or(false);
            if inline && !self.include_inline {
                continue;
            }

            let filename = part
                .attachment_name()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("attachment-{index}"));
            let safe = sanitize(&filename);
            let path = unique_path(&dir, &safe);

            fs::write(&path, part.contents())?;
            written.push(path.display().to_string());
        }

        if written.is_empty() {
            return printer.out(Message::new("No attachments to download"));
        }

        printer.out(Message::new(format!(
            "Downloaded {} attachment(s):\n  {}",
            written.len(),
            written.join("\n  ")
        )))
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
