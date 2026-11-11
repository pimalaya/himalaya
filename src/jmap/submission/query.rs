//! # JMAP submission query
//!
//! The `jmap submission query` command, an RFC 8621
//! `EmailSubmission/query` chained into an `EmailSubmission/get`.

use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use io_jmap::rfc8621::email_submission::{
    JmapEmailSubmission, JmapUndoStatus,
    query::{JmapEmailSubmissionFilter, JmapEmailSubmissionQueryOptions},
};
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, jmap::client::JmapClient};

/// How far along a submission is, and whether it can still be canceled.
#[derive(Clone, Debug, ValueEnum)]
pub enum UndoStatusArg {
    /// Not sent yet, so still cancelable.
    Pending,
    /// Sent, so no longer cancelable.
    Final,
    /// Canceled before it was sent.
    Canceled,
}

impl From<UndoStatusArg> for JmapUndoStatus {
    fn from(arg: UndoStatusArg) -> Self {
        match arg {
            UndoStatusArg::Pending => JmapUndoStatus::Pending,
            UndoStatusArg::Final => JmapUndoStatus::Final,
            UndoStatusArg::Canceled => JmapUndoStatus::Canceled,
        }
    }
}

/// Query JMAP email submissions (EmailSubmission/query + EmailSubmission/get).
#[derive(Debug, Parser)]
pub struct JmapSubmissionQueryCommand {
    /// Filter by undo status (`pending`, `final`, `canceled`).
    #[arg(long, value_name = "STATUS")]
    pub undo_status: Option<UndoStatusArg>,
    /// Filter by sent-before date (RFC 3339).
    #[arg(long, value_name = "DATE")]
    pub before: Option<String>,
    /// Filter by sent-after date (RFC 3339).
    #[arg(long, value_name = "DATE")]
    pub after: Option<String>,
    /// Number of submissions to display per page.
    #[arg(long, short = 's', value_name = "N", default_value = "10")]
    pub page_size: u64,
    /// Page index, starting from 1.
    #[arg(long, short, value_name = "N", default_value = "1")]
    pub page: u64,
}

impl JmapSubmissionQueryCommand {
    /// Queries the submissions and prints the page it returned.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let filter = {
            let f = JmapEmailSubmissionFilter {
                undo_status: self.undo_status.map(Into::into),
                before: self.before,
                after: self.after,
                ..Default::default()
            };

            let has_one = f.undo_status.is_some() || f.before.is_some() || f.after.is_some();

            if has_one { Some(f) } else { None }
        };

        let output = client.email_submission_query(JmapEmailSubmissionQueryOptions {
            filter,
            sort: None,
            position: Some(self.page.saturating_sub(1) * self.page_size),
            limit: Some(self.page_size),
        })?;

        printer.out(JmapSubmissionQueryOutput {
            submissions: output.submissions,
        })
    }
}

/// The `jmap submission query` output, the submissions of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JmapSubmissionQueryOutput {
    /// The submissions, in the order the server returned them.
    pub submissions: Vec<JmapEmailSubmission>,
}

impl fmt::Display for JmapSubmissionQueryOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header = ["ID", "EMAIL-ID", "IDENTITY-ID", "STATUS", "SENT-AT"];
        let mut columns = Columns::new(header);

        for submission in &self.submissions {
            let text = |value: &Option<String>| {
                sanitize::text(value.as_deref().unwrap_or_default()).into_owned()
            };
            let status = submission.undo_status.as_ref().map(ToString::to_string);

            columns.push([
                text(&submission.id),
                text(&submission.email_id),
                text(&submission.identity_id),
                status.unwrap_or_default(),
                text(&submission.send_at),
            ]);
        }

        write!(f, "{columns}")
    }
}
