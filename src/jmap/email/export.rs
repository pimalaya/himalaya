use std::net::TcpStream;

use anyhow::{anyhow, Result};
use clap::Parser;
use io_jmap::{client::JmapClient, rfc8621::capabilities::MAIL};
use pimalaya_cli::printer::{Message, Printer};
use pimalaya_stream::tls::upgrade_tls;
use secrecy::SecretString;
use url::Url;

use crate::jmap::{account::JmapAccount, session::JmapAuth};

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
        let auth: JmapAuth = account.backend.auth.clone().try_into()?;
        let http_auth: SecretString = auth.into();

        let mut client = account.new_jmap_client()?;

        let properties = Some(vec!["id".to_owned(), "blobId".to_owned()]);
        let output = client.email_get(vec![self.id.clone()], properties, false, false, 0)?;

        let session = client.session().expect("session loaded by new_jmap_client");
        let api_url = session.api_url.clone();
        let account_id = session
            .primary_accounts
            .get(MAIL)
            .map(|s| s.as_str())
            .unwrap_or("");

        let blob_id = output
            .emails
            .into_iter()
            .next()
            .and_then(|e| e.blob_id)
            .ok_or_else(|| anyhow!("Email `{}` not found or has no blobId", self.id))?;

        let download_url: Url = session
            .download_url
            .replace("{accountId}", account_id)
            .replace("{blobId}", &blob_id)
            .replace("{type}", "message%2Frfc822")
            .replace("{name}", "message.eml")
            .parse()?;

        let data = if same_authority(&api_url, &download_url) {
            client.blob_download(&download_url)?
        } else {
            let host = download_url.host_str().unwrap_or("localhost");
            let port = download_url.port_or_known_default().unwrap_or(443);
            let tcp = TcpStream::connect((host, port))?;
            let stream = upgrade_tls(host, tcp, &tls, &[b"http/1.1"])?;
            let mut download_client = JmapClient::new(stream, http_auth);
            download_client.blob_download(&download_url)?
        };

        printer.out(Message::new(String::from_utf8(data)?))
    }
}

fn same_authority(a: &Url, b: &Url) -> bool {
    a.host() == b.host() && a.port_or_known_default() == b.port_or_known_default()
}
