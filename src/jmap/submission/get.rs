use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::coroutines::email_submission_get::{
    GetJmapEmailSubmissions, GetJmapEmailSubmissionsResult,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::account::JmapAccount;

/// Get JMAP email submissions by ID (EmailSubmission/get).
#[derive(Debug, Parser)]
pub struct GetSubmissionCommand {
    /// Submission ID(s) to retrieve.
    #[arg(value_name = "SUBMISSION_ID", num_args = 1..)]
    pub ids: Vec<String>,
}

impl GetSubmissionCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine =
            GetJmapEmailSubmissions::new(jmap.context, Some(self.ids.clone()))?;
        let mut arg = None;

        let (submissions, not_found) = loop {
            match coroutine.resume(arg.take()) {
                GetJmapEmailSubmissionsResult::Io(io) => {
                    arg = Some(handle(&mut jmap.stream, io)?)
                }
                GetJmapEmailSubmissionsResult::Ok {
                    context,
                    submissions,
                    not_found,
                    ..
                } => {
                    jmap.context = context;
                    break (submissions, not_found);
                }
                GetJmapEmailSubmissionsResult::Err { err, .. } => bail!(err),
            }
        };

        for id in &not_found {
            printer.log(format!("Submission `{id}` not found."))?;
        }

        printer.out(serde_json::to_value(&submissions)?)
    }
}
