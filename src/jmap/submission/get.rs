use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::email_submission_get::{
    JmapEmailSubmissionGet, JmapEmailSubmissionGetResult,
};
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{account::JmapAccount, submission::query::SubmissionsTable};

const READ_BUFFER_SIZE: usize = 16 * 1024;

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
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let (submissions, not_found) = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailSubmissionGetResult::Ok {
                    submissions,
                    not_found,
                    ..
                } => break (submissions, not_found),
                JmapEmailSubmissionGetResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapEmailSubmissionGetResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapEmailSubmissionGetResult::Err(err) => bail!("{err}"),
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
