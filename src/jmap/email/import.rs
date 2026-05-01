use std::{
    collections::BTreeMap,
    io::{stdin, BufRead, IsTerminal, Read, Write},
};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::{
    rfc8620::blob_upload::{JmapBlobUpload, JmapBlobUploadResult},
    rfc8621::{
        capabilities::MAIL,
        email::EmailImport,
        email_import::{JmapEmailImport, JmapEmailImportResult},
    },
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};
use url::Url;

use crate::jmap::{account::JmapAccount, error::format_set_error};

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Import an RFC 5322 message into a mailbox (upload + Email/import).
///
/// Reads the raw message from stdin or as trailing arguments. Use
/// `--upload-only` to stop after the upload and print the blobId.
#[derive(Debug, Parser)]
pub struct JmapEmailImportCommand {
    /// Mailbox ID(s) to place the imported email in.
    #[arg(long, value_name = "MAILBOX-ID")]
    pub mailbox_id: Vec<String>,

    /// Keywords to set on the imported email (e.g. `$seen`).
    #[arg(long, value_name = "KEYWORD")]
    pub keyword: Vec<String>,

    /// Override the `receivedAt` timestamp (RFC 3339).
    #[arg(long, value_name = "DATE")]
    pub received_at: Option<String>,

    /// Only upload the blob and print the blobId; skip Email/import.
    #[arg(long)]
    pub upload_only: bool,

    /// The raw RFC 5322 message (headers + body). Read from stdin if omitted.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "message", value_name = "MESSAGE")]
    pub message: Vec<String>,
}

impl JmapEmailImportCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let tls = account.backend.tls.clone().try_into()?;
        let mut jmap = account.new_jmap_session()?;

        let data: Vec<u8> = if stdin().is_terminal() || printer.is_json() {
            self.message
                .join(" ")
                .replace('\r', "")
                .replace('\n', "\r\n")
                .into_bytes()
        } else {
            let lines: Vec<String> = stdin().lock().lines().map_while(Result::ok).collect();
            lines.join("\r\n").into_bytes()
        };

        let account_id = jmap
            .session
            .primary_accounts
            .get(MAIL)
            .map(|s| s.as_str())
            .unwrap_or("");
        let url: Url = jmap
            .session
            .upload_url
            .replace("{accountId}", account_id)
            .parse()?;

        let mut extra_stream = jmap.connect_if_different(&url, &tls)?;

        let mut coroutine = JmapBlobUpload::new(&jmap.http_auth, &url, "message/rfc822", data);
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let blob_id = loop {
            let stream = extra_stream.as_mut().unwrap_or(&mut jmap.stream);

            match coroutine.resume(arg.take()) {
                JmapBlobUploadResult::Ok { blob_id, .. } => break blob_id,
                JmapBlobUploadResult::WantsRead => {
                    let n = stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapBlobUploadResult::WantsWrite(bytes) => {
                    stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapBlobUploadResult::Err(err) => bail!("{err}"),
            }
        };

        if self.upload_only {
            return printer.out(Message::new(blob_id));
        }

        let mailbox_ids: BTreeMap<String, bool> =
            self.mailbox_id.into_iter().map(|m| (m, true)).collect();

        let keywords = if self.keyword.is_empty() {
            None
        } else {
            Some(self.keyword.iter().map(|kw| (kw.clone(), true)).collect())
        };

        let import = EmailImport {
            blob_id: blob_id.clone(),
            mailbox_ids,
            keywords,
            received_at: self.received_at,
        };

        let mut emails = BTreeMap::new();
        emails.insert(blob_id.clone(), import);

        let mut coroutine = JmapEmailImport::new(&jmap.session, &jmap.http_auth, emails)?;
        let mut arg: Option<&[u8]> = None;

        let not_created = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailImportResult::Ok { not_created, .. } => break not_created,
                JmapEmailImportResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapEmailImportResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapEmailImportResult::Err(err) => bail!("{err}"),
            }
        };

        if let Some(err) = not_created.get(&blob_id) {
            let mut msg = format!("Import JMAP email from blob `{blob_id}` error");
            msg.push_str(&format_set_error(err));
            bail!(msg);
        }

        printer.out(Message::new("Email successfully imported"))
    }
}
