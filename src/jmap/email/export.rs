use std::io::{Read, Write};

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::{
    rfc8620::blob_download::{JmapBlobDownload, JmapBlobDownloadResult},
    rfc8621::{
        capabilities::MAIL,
        email_get::{JmapEmailGet, JmapEmailGetResult},
    },
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};
use url::Url;

use crate::jmap::account::JmapAccount;

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Export a raw RFC 5322 message to stdout (Email/get + blob download).
///
/// Fetches the blobId via Email/get then downloads the raw message blob.
#[derive(Debug, Parser)]
pub struct JmapEmailExportCommand {
    /// The email ID to export.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl JmapEmailExportCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let tls = account.backend.tls.clone().try_into()?;
        let mut jmap = account.new_jmap_session()?;

        let properties = Some(vec!["id".to_owned(), "blobId".to_owned()]);

        let mut coroutine = JmapEmailGet::new(
            &jmap.session,
            &jmap.http_auth,
            vec![self.id.clone()],
            properties,
            false,
            false,
            0,
        )?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let emails = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailGetResult::Ok { emails, .. } => break emails,
                JmapEmailGetResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapEmailGetResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapEmailGetResult::Err(err) => bail!("{err}"),
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

        let mut url: Url = jmap
            .session
            .download_url
            .replace("{accountId}", account_id)
            .replace("{blobId}", &blob_id)
            .replace("{type}", "message%2Frfc822")
            .replace("{name}", "message.eml")
            .parse()?;

        let mut extra_stream = jmap.connect_if_different(&url, &tls)?;
        let mut coroutine = JmapBlobDownload::new(&jmap.http_auth, &url);
        let mut arg: Option<&[u8]> = None;

        let data = loop {
            match coroutine.resume(arg.take()) {
                JmapBlobDownloadResult::Ok { data, .. } => break data,
                JmapBlobDownloadResult::WantsRead => {
                    let stream = extra_stream.as_mut().unwrap_or(&mut jmap.stream);
                    let n = stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapBlobDownloadResult::WantsWrite(bytes) => {
                    let stream = extra_stream.as_mut().unwrap_or(&mut jmap.stream);
                    stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapBlobDownloadResult::WantsRedirect { url: new_url, .. } => {
                    url = new_url;
                    extra_stream = jmap.connect_if_different(&url, &tls)?;
                    coroutine = JmapBlobDownload::new(&jmap.http_auth, &url);
                    arg = None;
                }
                JmapBlobDownloadResult::Err(err) => bail!("{err}"),
            }
        };

        printer.out(Message::new(String::from_utf8(data)?))
    }
}
