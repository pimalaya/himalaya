use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::{
    coroutines::email_import::{ImportJmapEmail, ImportJmapEmailResult},
    types::email::EmailImport,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Import an RFC 5322 message blob into a mailbox (Email/import).
///
/// The blob must already be uploaded to the JMAP server.
#[derive(Debug, Parser)]
pub struct ImportEmailCommand {
    /// Blob ID of the RFC 5322 message to import.
    #[arg(value_name = "BLOB-ID")]
    pub blob_id: String,

    /// Mailbox ID(s) to place the imported email in.
    #[arg(long, value_name = "MAILBOX-ID", num_args = 0..)]
    pub mailbox_id: Vec<String>,

    /// Keywords to set on the imported email (e.g. `$seen`).
    #[arg(long, value_name = "KEYWORD", num_args = 0..)]
    pub keyword: Vec<String>,

    /// Override the `receivedAt` time (RFC 3339).
    #[arg(long, value_name = "DATE")]
    pub received_at: Option<String>,
}

impl ImportEmailCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mailbox_ids: HashMap<String, bool> =
            self.mailbox_id.iter().map(|m| (m.clone(), true)).collect();

        let keywords = if self.keyword.is_empty() {
            None
        } else {
            Some(self.keyword.iter().map(|kw| (kw.clone(), true)).collect())
        };

        let import = EmailImport {
            blob_id: self.blob_id.clone(),
            mailbox_ids,
            keywords,
            received_at: self.received_at,
        };

        let mut emails = HashMap::new();
        emails.insert(self.blob_id.clone(), import);

        let mut coroutine = ImportJmapEmail::new(jmap.context, emails)?;
        let mut arg = None;

        let not_created = loop {
            match coroutine.resume(arg.take()) {
                ImportJmapEmailResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                ImportJmapEmailResult::Ok { not_created, .. } => break not_created,
                ImportJmapEmailResult::Err { err, .. } => bail!(err),
            }
        };

        if let Some(err) = not_created.get(&self.blob_id) {
            let mut ctx = anyhow!("Failed to import email from blob `{}`", self.blob_id);

            if let Some(desc) = &err.description {
                ctx = anyhow!(desc.clone()).context(ctx);
            }

            bail!(ctx);
        }

        printer.out(Message::new("Email successfully imported from blob"))
    }
}
