use anyhow::Result;
use clap::Parser;
use log::warn;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::jmap::{
    client::JmapClient,
    email::query::{EmailsChars, EmailsColors, EmailsTable},
};

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
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = client.email_get(self.ids.clone(), Default::default())?;

        for id in output.not_found {
            warn!("email `{id}` not found, ignoring it");
        }

        let table = EmailsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            colors: EmailsColors {
                id: account.envelopes_list_table_id_color(),
                flags: account.envelopes_list_table_flags_color(),
                subject: account.envelopes_list_table_subject_color(),
                from: account.envelopes_list_table_from_color(),
                date: account.envelopes_list_table_date_color(),
            },
            chars: EmailsChars {
                unseen: account.envelopes_list_table_unseen_char(),
                flagged: account.envelopes_list_table_flagged_char(),
                attachment: account.envelopes_list_table_attachment_char(),
            },
            emails: output.emails,
        };

        printer.out(table)
    }
}
