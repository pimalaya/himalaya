use anyhow::{Result, anyhow};
use clap::Parser;
use io_jmap::{
    client::JmapClientStd,
    rfc8621::{
        MAIL_CAPABILITY,
        email::{JmapEmailProperty, get::JmapEmailGetOptions},
    },
};
use pimalaya_cli::printer::{Message, Printer};
use url::Url;

use crate::jmap::client::{JmapClient, jmap_http_auth};

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
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let opts = JmapEmailGetOptions {
            properties: Some(vec![JmapEmailProperty::Id, JmapEmailProperty::BlobId]),
            fetch_text_body_values: false,
            fetch_html_body_values: false,
            max_body_value_bytes: 0,
        };
        let output = client.email_get(vec![self.id.clone()], opts)?;

        let session = client
            .session()
            .expect("session loaded by build_jmap_client");
        let api_url = session.api_url.clone();
        let account_id = session
            .primary_accounts
            .get(MAIL_CAPABILITY)
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
            let tls = client
                .config
                .tls
                .clone()
                .into_tls(client.config.alpn.clone());
            let http_auth = jmap_http_auth(client.config.auth.clone())?;
            let mut download_client = JmapClientStd::connect(&download_url, &tls, http_auth)?;
            download_client.blob_download(&download_url)?
        };

        printer.out(Message::new(String::from_utf8(data)?))
    }
}

fn same_authority(a: &Url, b: &Url) -> bool {
    a.host() == b.host() && a.port_or_known_default() == b.port_or_known_default()
}
