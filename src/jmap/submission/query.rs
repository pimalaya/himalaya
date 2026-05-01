use std::{
    fmt,
    io::{Read, Write},
};

use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use comfy_table::{Cell, Row, Table};
use io_jmap::rfc8621::{
    email_submission::{EmailSubmission, EmailSubmissionFilter, UndoStatus},
    email_submission_query::{JmapEmailSubmissionQuery, JmapEmailSubmissionQueryResult},
};
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::jmap::account::JmapAccount;

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// CLI proxy for [`UndoStatus`].
#[derive(Clone, Debug, ValueEnum)]
pub enum UndoStatusArg {
    Pending,
    Final,
    Canceled,
}

impl From<UndoStatusArg> for UndoStatus {
    fn from(arg: UndoStatusArg) -> Self {
        match arg {
            UndoStatusArg::Pending => UndoStatus::Pending,
            UndoStatusArg::Final => UndoStatus::Final,
            UndoStatusArg::Canceled => UndoStatus::Canceled,
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
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let filter = {
            let f = EmailSubmissionFilter {
                undo_status: self.undo_status.map(Into::into),
                before: self.before,
                after: self.after,
                ..Default::default()
            };

            let has_one = f.undo_status.is_some() || f.before.is_some() || f.after.is_some();

            if has_one {
                Some(f)
            } else {
                None
            }
        };

        let mut coroutine = JmapEmailSubmissionQuery::new(
            &jmap.session,
            &jmap.http_auth,
            filter,
            None,
            Some(self.page.saturating_sub(1) * self.page_size),
            Some(self.page_size),
        )?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let submissions = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailSubmissionQueryResult::Ok { submissions, .. } => break submissions,
                JmapEmailSubmissionQueryResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapEmailSubmissionQueryResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapEmailSubmissionQueryResult::Err(err) => bail!("{err}"),
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
                    Cell::new(s.email_id.as_deref().unwrap_or("")),
                    Cell::new(s.identity_id.as_deref().unwrap_or("")),
                    Cell::new(
                        s.undo_status
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                    ),
                    Cell::new(s.send_at.as_deref().unwrap_or("")),
                ])
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
