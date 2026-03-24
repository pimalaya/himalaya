use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_jmap::{
    coroutines::email_submission_query::{
        QueryJmapEmailSubmissions, QueryJmapEmailSubmissionsResult,
    },
    types::email_submission::EmailSubmission,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::jmap::account::JmapAccount;

/// Query JMAP email submissions (EmailSubmission/query + EmailSubmission/get).
#[derive(Debug, Parser)]
pub struct QuerySubmissionCommand {
    /// Filter by undo status (`pending`, `final`, `canceled`).
    #[arg(long, value_name = "STATUS")]
    pub undo_status: Option<String>,

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

impl QuerySubmissionCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let filter = {
            use io_jmap::types::email_submission::EmailSubmissionFilter;
            let f = EmailSubmissionFilter {
                undo_status: self.undo_status,
                before: self.before,
                after: self.after,
                ..Default::default()
            };
            let has_one = f.undo_status.is_some() || f.before.is_some() || f.after.is_some();
            if has_one { Some(f) } else { None }
        };

        let mut coroutine = QueryJmapEmailSubmissions::new(
            jmap.context,
            filter,
            None,
            Some(self.page.saturating_sub(1) * self.page_size),
            Some(self.page_size),
        )?;
        let mut arg = None;

        let submissions = loop {
            match coroutine.resume(arg.take()) {
                QueryJmapEmailSubmissionsResult::Io(io) => {
                    arg = Some(handle(&mut jmap.stream, io)?)
                }
                QueryJmapEmailSubmissionsResult::Ok { context, submissions, .. } => {
                    jmap.context = context;
                    break submissions;
                }
                QueryJmapEmailSubmissionsResult::Err { err, .. } => bail!(err),
            }
        };

        let table = SubmissionsTable {
            preset: account.table_preset,
            submissions,
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct SubmissionsTable {
    #[serde(skip)]
    pub preset: String,
    pub submissions: Vec<EmailSubmission>,
}

impl fmt::Display for SubmissionsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("EMAIL-ID"),
                Cell::new("IDENTITY-ID"),
                Cell::new("STATUS"),
                Cell::new("SENT-AT"),
            ]))
            .add_rows(self.submissions.iter().map(|s| {
                Row::from([
                    Cell::new(s.id.as_deref().unwrap_or("")),
                    Cell::new(&s.email_id),
                    Cell::new(&s.identity_id),
                    Cell::new(s.undo_status.as_deref().unwrap_or("")),
                    Cell::new(s.send_at.as_deref().unwrap_or("")),
                ])
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
