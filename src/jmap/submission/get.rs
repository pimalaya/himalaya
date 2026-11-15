//! # JMAP submission get
//!
//! The `jmap submission get` command, RFC 8621 `EmailSubmission/get`.

use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::email_submission::get::JmapEmailSubmissionGetOptions;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    error::{ErrorCode, partial_when},
    jmap::{
        backend::not_found_errors, client::JmapClient, submission::query::JmapSubmissionQueryOutput,
    },
};

/// Get JMAP email submissions by ID (EmailSubmission/get).
///
/// An id the server does not find is an entry of `errors`, coded
/// `submission-not-found`, and exits 3.
#[derive(Debug, Parser)]
pub struct JmapSubmissionGetCommand {
    /// Submission ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapSubmissionGetCommand {
    /// Fetches the submissions and lists them, then the error of each id
    /// not found.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = client.email_submission_get(JmapEmailSubmissionGetOptions {
            ids: Some(self.ids.clone()),
        })?;

        let submissions = JmapSubmissionQueryOutput {
            account: account.name().to_string(),
            page: None,
            submissions: output.submissions,
            next_page: None,
            errors: not_found_errors(output.not_found, ErrorCode::SubmissionNotFound),
        };

        printer.out(&submissions)?;
        partial_when(&submissions.errors)
    }
}
