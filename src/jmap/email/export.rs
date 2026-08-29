//! # JMAP email export
//!
//! The `jmap email export` command, an `Email/get` for the blob id then a
//! download of that blob.

use anyhow::{Result, anyhow};
use clap::Parser;
use io_jmap::rfc8621::{
    JMAP_MAIL_CAPABILITY,
    email::{JmapEmailProperty, get::JmapEmailGetOptions},
};
use pimalaya_cli::printer::{Message, Printer};
use url::Url;

use crate::jmap::client::JmapClient;

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
    /// Downloads the raw message blob and writes it out.
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
        let account_id = session
            .primary_accounts
            .get(JMAP_MAIL_CAPABILITY)
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

        let data = client.download_blob(&download_url)?;

        printer.out(Message::new(String::from_utf8(data)?))
    }
}
