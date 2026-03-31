use std::{
    collections::HashMap,
    io::{stdin, BufRead, IsTerminal},
};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::{
    rfc8620::coroutines::blob_upload::{JmapBlobUpload, JmapBlobUploadResult},
    rfc8620::types::session::capabilities,
    rfc8621::coroutines::email_import::{JmapEmailImport, JmapEmailImportResult},
    rfc8621::types::email::EmailImport,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};
use url::Url;

use crate::jmap::account::JmapAccount;

/// Import an RFC 5322 message into a mailbox (upload + Email/import).
///
/// Reads the raw message from stdin or as trailing arguments. Use
/// `--upload-only` to stop after the upload and print the blobId.
#[derive(Debug, Parser)]
pub struct ImportEmailCommand {
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

impl ImportEmailCommand {
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
            .get(capabilities::MAIL)
            .map(|s| s.as_str())
            .unwrap_or("");
        let url: Url = jmap
            .session
            .upload_url
            .replace("{accountId}", account_id)
            .parse()?;

        let mut extra_stream = jmap.connect_if_different(&url, &tls)?;
        let upload_stream = extra_stream.as_mut().unwrap_or(&mut jmap.stream);

        let mut coroutine = JmapBlobUpload::new(&jmap.http_auth, &url, "message/rfc822", data)?;
        let mut arg = None;

        let blob_id = loop {
            match coroutine.resume(arg.take()) {
                JmapBlobUploadResult::Io { io } => arg = Some(handle(&mut *upload_stream, io)?),
                JmapBlobUploadResult::Ok { blob_id, .. } => {
                    break blob_id;
                }
                JmapBlobUploadResult::Err { err, .. } => bail!(err),
            }
        };

        if self.upload_only {
            return printer.out(Message::new(blob_id));
        }

        let mailbox_ids: HashMap<String, bool> =
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

        let mut emails = HashMap::new();
        emails.insert(blob_id.clone(), import);

        let mut coroutine = JmapEmailImport::new(&jmap.session, &jmap.http_auth, emails)?;
        let mut arg = None;

        let not_created = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailImportResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapEmailImportResult::Ok { not_created, .. } => break not_created,
                JmapEmailImportResult::Err { err, .. } => bail!(err),
            }
        };

        if let Some(err) = not_created.get(&blob_id) {
            let mut msg = format!("Import JMAP email from blob `{blob_id}` error");

            if !err.properties.is_empty() {
                msg.push_str(": invalid properties `");
                msg.push_str(&err.properties.join("`, `"));
                msg.push('`');
            }

            if let Some(desc) = &err.description {
                msg.push_str(" (");
                msg.push_str(desc.to_lowercase().trim_end_matches(['.', '\n']));
                msg.push(')');
            }

            bail!(msg);
        }

        printer.out(Message::new("Email successfully imported"))
    }
}
