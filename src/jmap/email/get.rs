use anyhow::Result;
use clap::Parser;
use log::warn;
use pimalaya_cli::printer::Printer;

use crate::jmap::{client::JmapClient, email::query::EmailsTable};

/// Get JMAP emails by ID (Email/get).
///
/// Fetches and displays email envelopes as a table.
#[derive(Debug, Parser)]
pub struct JmapEmailGetCommand {
    /// The email ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapEmailGetCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: JmapClient) -> Result<()> {
        let output = client.email_get(self.ids.clone(), None, false, false, 0)?;

        for id in output.not_found {
            warn!("email `{id}` not found, ignoring it");
        }

        let table = EmailsTable {
            preset: client.account.table_preset().to_string(),
            arrangement: client.account.table_arrangement(),
            emails: output.emails,
        };

        printer.out(table)
    }
}
