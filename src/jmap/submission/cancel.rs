use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::rfc8621::coroutines::email_submission_cancel::{
    JmapEmailSubmissionCancel, JmapEmailSubmissionCancelResult,
};
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

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
        let mut arg = None;

        let not_updated = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailSubmissionCancelResult::Io { io } => {
                    arg = Some(handle(&mut jmap.stream, io)?)
                }
                JmapEmailSubmissionCancelResult::Ok { not_updated, .. } => break not_updated,
                JmapEmailSubmissionCancelResult::Err { err, .. } => bail!(err),
            }
        };

        if !not_updated.is_empty() {
            let mut msg = String::from("Cancel submission(s) error");

            for (id, err) in &not_updated {
                msg.push_str(&format!("\n  `{id}`"));

                if !err.properties.is_empty() {
                    msg.push_str(": invalid properties `");
                    msg.push_str(&err.properties.join("`, `"));
                    msg.push('`');
                }

                if let Some(desc) = &err.description {
                    msg.push_str(" (");
                    msg.push_str(desc.to_lowercase().trim_end_matches(['.', '\n']));
                    msg.push(')');
                }
            }

            bail!(msg);
        }

        printer.out(Message::new(format!(
            "{} submission(s) canceled.",
            self.ids.len()
        )))
    }
}
