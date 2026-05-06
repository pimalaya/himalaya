use anyhow::Result;
use clap::Parser;
use log::warn;
use pimalaya_cli::printer::Printer;

use crate::jmap::{client::JmapClient, submission::query::SubmissionsTable};

/// Get JMAP email submissions by ID (EmailSubmission/get).
#[derive(Debug, Parser)]
pub struct JmapSubmissionGetCommand {
    /// Submission ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapSubmissionGetCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: JmapClient) -> Result<()> {
        let output = client.email_submission_get(Some(self.ids.clone()))?;

        for id in output.not_found {
            warn!("submission `{id}` not found, ignoring it");
        }

        let table = SubmissionsTable {
            preset: client.account.table_preset().to_string(),
            submissions: output.submissions,
        };

        printer.out(table)
    }
}
