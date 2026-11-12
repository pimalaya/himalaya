//! # JMAP submission get
//!
//! The `jmap submission get` command, RFC 8621 `EmailSubmission/get`.

use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::email_submission::get::JmapEmailSubmissionGetOptions;
use log::warn;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    jmap::{client::JmapClient, submission::query::JmapSubmissionQueryOutput},
};

/// Get JMAP email submissions by ID (EmailSubmission/get).
#[derive(Debug, Parser)]
pub struct JmapSubmissionGetCommand {
    /// Submission ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapSubmissionGetCommand {
    /// Fetches the submissions and lists them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = client.email_submission_get(JmapEmailSubmissionGetOptions {
            ids: Some(self.ids.clone()),
        })?;

        for id in output.not_found {
            warn!("submission `{id}` not found, ignoring it");
        }

        printer.out(JmapSubmissionQueryOutput {
            account: account.name().to_string(),
            page: None,
            submissions: output.submissions,
        })
    }
}
