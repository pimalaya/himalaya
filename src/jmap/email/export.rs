use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::{
    rfc8620::{
        coroutines::blob_download::{JmapBlobDownload, JmapBlobDownloadResult},
        types::session::capabilities::MAIL,
    },
    rfc8621::coroutines::email_get::{JmapEmailGet, JmapEmailGetResult},
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};
use url::Url;

use crate::jmap::account::JmapAccount;

/// Export a raw RFC 5322 message to stdout (Email/get + blob download).
///
/// Fetches the blobId via Email/get then downloads the raw message blob.
#[derive(Debug, Parser)]
pub struct ExportEmailCommand {
    /// The email ID to export.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl ExportEmailCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let tls = account.backend.tls.clone().try_into()?;
        let mut jmap = account.new_jmap_session()?;

        let properties = Some(vec!["id".to_owned(), "blobId".to_owned()]);

        let mut arg = None;
        let mut coroutine = JmapEmailGet::new(
            &jmap.session,
            &jmap.http_auth,
            vec![self.id.clone()],
            properties,
            false,
            false,
            0,
        )?;

        let emails = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailGetResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapEmailGetResult::Ok { emails, .. } => {
                    break emails;
                }
                JmapEmailGetResult::Err { err, .. } => bail!(err),
            }
        };

        let account_id = jmap
            .session
            .primary_accounts
            .get(MAIL)
            .map(|s| s.as_str())
            .unwrap_or("");
        let blob_id = emails
            .into_iter()
            .next()
            .and_then(|e| e.blob_id)
            .ok_or_else(|| anyhow!("Email `{}` not found or has no blobId", self.id))?;

        let url: Url = jmap
            .session
            .download_url
            .replace("{accountId}", account_id)
            .replace("{blobId}", &blob_id)
            .replace("{type}", "message%2Frfc822")
            .replace("{name}", "message.eml")
            .parse()?;

        let mut stream = jmap.connect_if_different(&url, &tls)?;
        let stream = stream.as_mut().unwrap_or(&mut jmap.stream);

        let mut coroutine = JmapBlobDownload::new(&jmap.http_auth, &url)?;
        let mut arg = None;

        let data = loop {
            match coroutine.resume(arg.take()) {
                JmapBlobDownloadResult::Io { io } => arg = Some(handle(&mut *stream, io)?),
                JmapBlobDownloadResult::Ok { data, .. } => break data,
                JmapBlobDownloadResult::Err { err, .. } => bail!(err),
            }
        };

        printer.out(Message::new(String::from_utf8(data)?))
    }
}
