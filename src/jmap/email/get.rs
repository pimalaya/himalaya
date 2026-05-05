use anyhow::Result;
use clap::Parser;
use log::warn;
use pimalaya_cli::printer::Printer;

use crate::jmap::{account::JmapAccount, email::query::EmailsTable};

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
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut client = account.new_jmap_client()?;
        let output = client.email_get(self.ids.clone(), None, false, false, 0)?;

        for id in output.not_found {
            warn!("email `{id}` not found, ignoring it");
        }

        let table = EmailsTable {
            preset: account.table_preset,
            arrangement: account.table_arrangement,
            emails: output.emails,
        };

        printer.out(table)
    }
}
