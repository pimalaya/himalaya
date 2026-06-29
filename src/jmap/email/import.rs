use std::collections::BTreeMap;

use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::{
    client::JmapClientStd,
    rfc8621::{MAIL_CAPABILITY, email::JmapEmailImportArgs},
};
use pimalaya_cli::printer::{Message, Printer};
use url::Url;

use crate::{
    jmap::{
        client::{JmapClient, jmap_http_auth},
        error::format_set_error,
    },
    shared::message::arg::MessageArg,
};

/// Import an RFC 5322 message into a mailbox (upload + Email/import).
///
/// The message can be passed as a positional file path, an inline
/// raw string, or piped via stdin (see [`MessageArg`] for resolution
/// order). Use `--upload-only` to stop after the upload and print
/// the blobId.
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

    #[command(flatten)]
    pub message: MessageArg,
}

impl JmapEmailImportCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let data = self.message.parse()?.into_bytes();

        let session = client
            .session()
            .expect("session loaded by build_jmap_client");
        let api_url = session.api_url.clone();
        let account_id = session
            .primary_accounts
            .get(MAIL_CAPABILITY)
            .map(|s| s.as_str())
            .unwrap_or("");
        let upload_url: Url = session
            .upload_url
            .replace("{accountId}", account_id)
            .parse()?;

        let blob_id = if same_authority(&api_url, &upload_url) {
            client
                .blob_upload(&upload_url, "message/rfc822", data)?
                .blob_id
        } else {
            let tls = client
                .config
                .tls
                .clone()
                .into_tls(client.config.alpn.clone());
            let http_auth = jmap_http_auth(client.config.auth.clone())?;
            let mut upload_client = JmapClientStd::connect(&upload_url, &tls, http_auth)?;
            upload_client
                .blob_upload(&upload_url, "message/rfc822", data)?
                .blob_id
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

        let import = JmapEmailImportArgs {
            blob_id: blob_id.clone(),
            mailbox_ids,
            keywords,
            received_at: self.received_at,
        };

        let mut emails = BTreeMap::new();
        emails.insert(blob_id.clone(), import);

        let output = client.email_import(emails)?;

        if let Some(err) = output.not_created.get(&blob_id) {
            let mut msg = format!("Import JMAP email from blob `{blob_id}` error");
            msg.push_str(&format_set_error(err));
            bail!(msg);
        }

        printer.out(Message::new("Email successfully imported"))
    }
}

fn same_authority(a: &Url, b: &Url) -> bool {
    a.host() == b.host() && a.port_or_known_default() == b.port_or_known_default()
}
