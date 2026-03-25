use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::coroutines::email_submission_get::{
    GetJmapEmailSubmissions, GetJmapEmailSubmissionsResult,
};
use io_stream::runtimes::std::handle;
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{account::JmapAccount, submission::query::SubmissionsTable};

/// Get JMAP email submissions by ID (EmailSubmission/get).
#[derive(Debug, Parser)]
pub struct GetSubmissionCommand {
    /// Submission ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl GetSubmissionCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine = GetJmapEmailSubmissions::new(jmap.context, Some(self.ids.clone()))?;
        let mut arg = None;

        let (submissions, not_found) = loop {
            match coroutine.resume(arg.take()) {
                GetJmapEmailSubmissionsResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                GetJmapEmailSubmissionsResult::Ok {
                    submissions,
                    not_found,
                    ..
                } => break (submissions, not_found),
                GetJmapEmailSubmissionsResult::Err { err, .. } => bail!(err),
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
