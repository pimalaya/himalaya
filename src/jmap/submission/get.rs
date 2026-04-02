use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::coroutines::email_submission_get::{
    JmapEmailSubmissionGet, JmapEmailSubmissionGetResult,
};
use io_socket::runtimes::std_stream::handle;
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;

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
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine =
            JmapEmailSubmissionGet::new(&jmap.session, &jmap.http_auth, Some(self.ids.clone()))?;
        let mut arg = None;

        let (submissions, not_found) = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailSubmissionGetResult::Io { io } => {
                    arg = Some(handle(&mut jmap.stream, io)?)
                }
                JmapEmailSubmissionGetResult::Ok {
                    submissions,
                    not_found,
                    ..
                } => break (submissions, not_found),
                JmapEmailSubmissionGetResult::Err { err, .. } => bail!(err),
            }
        };

        for id in not_found {
            warn!("submission `{id}` not found, ignoring it");
        }

        let table = SubmissionsTable {
            preset: account.table_preset,
            submissions,
        };

        printer.out(table)
    }
}
