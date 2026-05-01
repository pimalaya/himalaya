use std::io::{Read, Write};

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::rfc8621::email_submission_cancel::{
    JmapEmailSubmissionCancel, JmapEmailSubmissionCancelResult,
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::{account::JmapAccount, error::format_set_error};

const READ_BUFFER_SIZE: usize = 16 * 1024;

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
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine =
            JmapEmailSubmissionCancel::new(&jmap.session, &jmap.http_auth, self.ids.clone())
                .map_err(|e| anyhow!("{e}"))?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let not_updated = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailSubmissionCancelResult::Ok { not_updated, .. } => break not_updated,
                JmapEmailSubmissionCancelResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapEmailSubmissionCancelResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapEmailSubmissionCancelResult::Err(err) => bail!("{err}"),
            }
        };

        if !not_updated.is_empty() {
            let mut msg = String::from("Cancel submission(s) error");

            for (id, err) in &not_updated {
                msg.push_str(&format!("\n  `{id}`"));
                msg.push_str(&format_set_error(err));
            }

            bail!(msg);
        }

        printer.out(Message::new(format!(
            "{} submission(s) canceled.",
            self.ids.len()
        )))
    }
}
