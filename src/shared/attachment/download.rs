//! # Attachment download
//!
//! The `attachment download` command, writing the attachment parts of one
//! message to disk.

use std::{
    collections::BTreeSet,
    fmt,
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write, stdout},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use base64::{Engine, prelude::BASE64_STANDARD};
use clap::Parser;
use mail_parser::Message;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    shared::{
        attachment::list::{self, AttachmentColors, Attachments},
        client::EmailClient,
        mailbox::arg::MailboxArg,
        message::part,
    },
};

/// Download the attachments of one message to disk.
///
/// An attachment id is the MIME part position `attachment list` and
/// `message read` report, inline parts included. A name colliding with an
/// existing file is suffixed rather than overwriting it.
///
/// `--stdout` writes the decoded bytes of one part to the standard output
/// instead, and touches no file.
#[derive(Debug, Parser)]
pub struct AttachmentDownloadCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Identifier of the message.
    #[arg(value_name = "MESSAGE-ID")]
    pub message_id: String,
    /// Identifiers of the attachments to download, all of them when none
    /// is given.
    #[arg(value_name = "ATTACHMENT-ID", num_args = 0..)]
    pub attachment_ids: Vec<String>,
    /// Destination directory, overriding the configured `downloads-dir`.
    #[arg(long, short, value_name = "PATH", conflicts_with = "stdout")]
    pub dir: Option<PathBuf>,
    /// Write the decoded bytes of one part to stdout, and nothing else.
    ///
    /// It takes exactly one id, which may be any part `message read`
    /// lists, a body included. No file is written. With the global
    /// `--json` flag the part comes out as `{id, mime, filename, size,
    /// data}` instead, `data` holding the bytes in base64.
    #[arg(long)]
    pub stdout: bool,
}

impl AttachmentDownloadCommand {
    /// The one part id `--stdout` reads, `None` without the flag.
    fn stdout_part_id(&self) -> Result<Option<String>> {
        match (self.stdout, self.attachment_ids.as_slice()) {
            (false, _) => Ok(None),
            (true, [id]) => Ok(Some(id.clone())),
            (true, _) => bail!("`--stdout` takes exactly one attachment id"),
        }
    }

    /// Fetches the message, writes the wanted parts and tables what
    /// landed where.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let part_id = self.stdout_part_id()?;

        let mailbox = self.mailbox.resolve(account);
        let raw = client.get_message(&mailbox, &self.message_id, false)?;
        let message = part::parse(&raw)?;

        if let Some(id) = part_id {
            let Some(part) = PartBytes::new(&message, &id) else {
                bail!("No part with id {id} on message `{}`", self.message_id);
            };

            return part.print(printer);
        }

        let dir = self.dir.clone().unwrap_or_else(|| account.downloads_dir());

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        let wanted_all = self.attachment_ids.is_empty();
        let mut remaining: BTreeSet<String> = self.attachment_ids.iter().cloned().collect();
        let mut written = Vec::new();

        for mut attachment in list::attachments(&message) {
            if !wanted_all && !remaining.remove(&attachment.id) {
                continue;
            }

            let on_disk_name = attachment
                .filename
                .clone()
                .unwrap_or_else(|| format!("attachment-{}", attachment.id));
            let safe = sanitize(&on_disk_name);
            let Some(leaf) = part::find(&message, &attachment.id) else {
                continue;
            };
            let (path, mut file) = create_unique_file(&dir, &safe)?;
            file.write_all(&part::bytes(&message, leaf.part))
                .with_context(|| format!("Write attachment to {}", path.display()))?;

            attachment.path = Some(path.display().to_string());
            written.push(attachment);
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

        printer.out(AttachmentDownloadOutput::Written(attachments))
    }
}

/// The `attachment download` output: the table of the parts written, or
/// under `--stdout --json` the one part read.
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum AttachmentDownloadOutput {
    Written(Attachments),
    Part(PartBytes),
}

impl fmt::Display for AttachmentDownloadOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Written(attachments) => attachments.fmt(f),
            Self::Part(part) => f.write_str(&part.data),
        }
    }
}

/// One part read by `attachment download --stdout --json`.
#[derive(Serialize, JsonSchema)]
pub struct PartBytes {
    /// The id of the part, as `message read` and `attachment list` give
    /// it.
    pub id: String,
    /// The MIME type, lowercased.
    pub mime: String,
    /// The file name, RFC 2231 and RFC 2047 decoded; `null` when the part
    /// names none.
    pub filename: Option<String>,
    /// The size of the decoded bytes.
    pub size: u64,
    /// The decoded bytes, in base64.
    pub data: String,
    #[serde(skip)]
    bytes: Vec<u8>,
}

impl PartBytes {
    /// The leaf part of a message carrying the given id, with its bytes;
    /// `None` for an unknown id or a container's.
    pub fn new(message: &Message, id: &str) -> Option<Self> {
        let leaf = part::find(message, id)?;
        let bytes = part::bytes(message, leaf.part).into_owned();
        Some(Self {
            id: leaf.id,
            mime: part::mime(leaf.part),
            filename: part::filename(message, leaf.part),
            size: bytes.len() as u64,
            data: BASE64_STANDARD.encode(&bytes),
            bytes,
        })
    }

    /// Writes the bytes to stdout and nothing else, or under `--json`
    /// the part as `{id, mime, filename, size, data}`: what `attachment
    /// download --stdout` and `message parse --part` print.
    pub fn print(self, printer: &mut impl Printer) -> Result<()> {
        if printer.is_json() {
            return printer.out(AttachmentDownloadOutput::Part(self));
        }

        self.write_stdout()
    }

    /// Writes the decoded bytes to stdout, and nothing else.
    pub fn write_stdout(&self) -> Result<()> {
        let mut out = stdout().lock();
        out.write_all(&self.bytes)?;
        out.flush()?;
        Ok(())
    }
}

/// Strips path separators and parent traversals, so a hostile filename
/// header cannot escape the download directory.
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

/// Creates an unoccupied file, suffixing its name and refusing symlinks.
fn create_unique_file(dir: &Path, name: &str) -> Result<(PathBuf, File)> {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };

    for n in 0..1024 {
        let candidate = if n == 0 {
            dir.join(name)
        } else {
            dir.join(format!("{stem} ({n}){ext}"))
        };
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("Create attachment at {}", candidate.display()));
            }
        }
    }
    bail!(
        "No unused attachment name for `{name}` in {}",
        dir.display()
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Write,
        sync::{Arc, Barrier},
        thread,
    };

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    use base64::{Engine, prelude::BASE64_STANDARD};
    use clap::Parser;

    use super::{AttachmentDownloadCommand, PartBytes, create_unique_file, sanitize};
    use crate::shared::message::part;

    const MIXED: &[u8] = b"Content-Type: multipart/mixed; boundary=\"b\"\r\n\
        \r\n\
        --b\r\n\
        Content-Type: text/plain\r\n\
        \r\n\
        body\r\n\
        --b\r\n\
        Content-Type: application/octet-stream; name=\"blob.bin\"\r\n\
        Content-Transfer-Encoding: base64\r\n\
        \r\n\
        AAH//gAKDQ==\r\n\
        --b\r\n\
        Content-Type: text/calendar; charset=windows-1252; method=REQUEST\r\n\
        Content-Transfer-Encoding: quoted-printable\r\n\
        \r\n\
        SUMMARY:Caf=E9 =80 5\r\n\
        --b--\r\n";

    #[test]
    fn a_collision_does_not_replace_existing_content() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("report.txt");
        fs::write(&original, b"original").unwrap();
        let (path, mut file) = create_unique_file(dir.path(), "report.txt").unwrap();
        file.write_all(b"attachment").unwrap();
        assert_eq!(path, dir.path().join("report (1).txt"));
        assert_eq!(fs::read(original).unwrap(), b"original");
        assert_eq!(fs::read(path).unwrap(), b"attachment");
    }

    #[cfg(unix)]
    #[test]
    fn a_dangling_symlink_is_occupied_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("unexpected.txt");
        symlink(&target, dir.path().join("report.txt")).unwrap();
        let (path, mut file) = create_unique_file(dir.path(), "report.txt").unwrap();
        file.write_all(b"attachment").unwrap();
        assert!(!target.exists());
        assert_eq!(path, dir.path().join("report (1).txt"));
        assert_eq!(fs::read(path).unwrap(), b"attachment");
    }

    #[test]
    fn exhausted_names_fail_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("report.txt"), b"original").unwrap();
        for n in 1..1024 {
            fs::write(dir.path().join(format!("report ({n}).txt")), b"original").unwrap();
        }
        assert!(create_unique_file(dir.path(), "report.txt").is_err());
        for entry in fs::read_dir(dir.path()).unwrap() {
            assert_eq!(fs::read(entry.unwrap().path()).unwrap(), b"original");
        }
    }

    #[test]
    fn concurrent_downloads_get_distinct_files() {
        let dir = tempfile::tempdir().unwrap();
        let barrier = Arc::new(Barrier::new(8));
        let writers: Vec<_> = (0..8)
            .map(|n| {
                let dir = dir.path().to_path_buf();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    let (path, mut file) = create_unique_file(&dir, "report.txt").unwrap();
                    let bytes = format!("attachment {n}");
                    file.write_all(bytes.as_bytes()).unwrap();
                    (path, bytes)
                })
            })
            .collect();
        let results: Vec<_> = writers
            .into_iter()
            .map(|writer| writer.join().unwrap())
            .collect();
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 8);
        for (path, bytes) in results {
            assert_eq!(fs::read(path).unwrap(), bytes.as_bytes());
        }
    }

    #[test]
    fn stdout_bytes_are_the_decoded_part() {
        let message = part::parse(MIXED).unwrap();

        let blob = PartBytes::new(&message, "3").unwrap();
        assert_eq!(blob.bytes, b"\x00\x01\xff\xfe\x00\x0a\x0d");
        assert_eq!(blob.size, 7);
        assert_eq!(blob.mime, "application/octet-stream");
        assert_eq!(blob.filename.as_deref(), Some("blob.bin"));
        assert_eq!(BASE64_STANDARD.decode(&blob.data).unwrap(), blob.bytes);

        let invite = PartBytes::new(&message, "4").unwrap();
        assert_eq!(invite.bytes, b"SUMMARY:Caf\xe9 \x80 5");
        assert_eq!(invite.filename, None);

        // NOTE: a body is a part like any other, and the container none.
        assert_eq!(PartBytes::new(&message, "2").unwrap().bytes, b"body");
        assert!(PartBytes::new(&message, "1").is_none());
        assert!(PartBytes::new(&message, "5").is_none());
    }

    #[test]
    fn stdout_bytes_match_the_size_message_read_reports() {
        let message = part::parse(MIXED).unwrap();
        for leaf in part::leaves(&message) {
            let read = PartBytes::new(&message, &leaf.id).unwrap();
            assert_eq!(read.size, part::bytes(&message, leaf.part).len() as u64);
        }
    }

    #[test]
    fn stdout_takes_exactly_one_id() {
        let parse = |args: &[&str]| {
            AttachmentDownloadCommand::try_parse_from([&["download"], args].concat())
                .unwrap()
                .stdout_part_id()
        };

        assert_eq!(
            parse(&["7", "3", "--stdout"]).unwrap().as_deref(),
            Some("3")
        );
        assert!(parse(&["7", "--stdout"]).is_err());
        assert!(parse(&["7", "3", "4", "--stdout"]).is_err());
        assert_eq!(parse(&["7", "3", "4"]).unwrap(), None);
        assert!(
            AttachmentDownloadCommand::try_parse_from([
                "download", "7", "3", "--stdout", "-d", "x"
            ])
            .is_err()
        );
    }

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
