use anyhow::Result;
use clap::Parser;
use log::warn;
use pimalaya_cli::printer::Printer;

use crate::jmap::{account::JmapAccount, submission::query::SubmissionsTable};

/// Get JMAP email submissions by ID (EmailSubmission/get).
#[derive(Debug, Parser)]
pub struct JmapSubmissionGetCommand {
    /// Submission ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapSubmissionGetCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut client = account.new_jmap_client()?;
        let output = client.email_submission_get(Some(self.ids.clone()))?;

        for id in output.not_found {
            warn!("submission `{id}` not found, ignoring it");
        }

        let table = SubmissionsTable {
            preset: account.table_preset,
            submissions: output.submissions,
        };

        printer.out(table)
    }
}
