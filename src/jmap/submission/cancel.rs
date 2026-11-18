//! # JMAP submission cancel
//!
//! The `jmap submission cancel` command, an RFC 8621
//! `EmailSubmission/set` update setting the undo status.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    error::ErrorCode,
    jmap::{client::JmapClient, set::JmapSetReceipt},
};

/// Cancel (undo) a pending JMAP email submission (EmailSubmission/set).
///
/// Only submissions with `undoStatus: "pending"` can be canceled.
/// The server may reject this if the message has already been sent.
#[derive(Debug, Parser)]
pub struct JmapSubmissionCancelCommand {
    /// Submission ID(s) to cancel.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapSubmissionCancelCommand {
    /// Sets the undo status of the submission to canceled.
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let output = client.email_submission_cancel(self.ids)?;

        JmapSetReceipt::new(
            "JMAP submissions canceled",
            output.updated.into_keys().collect(),
            output.not_updated,
            Some(ErrorCode::SubmissionNotFound),
        )
        .print(printer)
    }
}
