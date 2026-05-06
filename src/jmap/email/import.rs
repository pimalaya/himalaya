use std::{
    collections::BTreeMap,
    io::{stdin, BufRead, IsTerminal},
    net::TcpStream,
};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::{
    client::JmapClient as InnerJmapClient,
    rfc8621::{capabilities::MAIL, email::EmailImport},
};
use pimalaya_cli::printer::{Message, Printer};
use pimalaya_stream::std::tls::upgrade_tls;
use secrecy::SecretString;
use url::Url;

use crate::jmap::{client::JmapClient, error::format_set_error, session::JmapAuth};

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
    pub fn execute(self, printer: &mut impl Printer, mut client: JmapClient) -> Result<()> {
        let tls = client.config.tls.clone().try_into()?;
        let auth: JmapAuth = client.config.auth.clone().try_into()?;
        let http_auth: SecretString = auth.into();

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

        let session = client.session().expect("session loaded by new_jmap_client");
        let api_url = session.api_url.clone();
        let account_id = session
            .primary_accounts
            .get(MAIL)
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
            let host = upload_url.host_str().unwrap_or("localhost");
            let port = upload_url.port_or_known_default().unwrap_or(443);
            let tcp = TcpStream::connect((host, port))?;
            let stream = upgrade_tls(host, tcp, &tls, &[b"http/1.1"])?;
            let mut upload_client = InnerJmapClient::new(stream, http_auth);
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

        let import = EmailImport {
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
