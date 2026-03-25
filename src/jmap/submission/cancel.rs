use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::coroutines::email_submission_cancel::{
    CancelJmapEmailSubmissions, CancelJmapEmailSubmissionsResult,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Cancel (undo) a pending JMAP email submission (EmailSubmission/set).
///
/// Only submissions with `undoStatus: "pending"` can be canceled.
/// The server may reject this if the message has already been sent.
#[derive(Debug, Parser)]
pub struct CancelSubmissionCommand {
    /// Submission ID(s) to cancel.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl CancelSubmissionCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine =
            CancelJmapEmailSubmissions::new(jmap.context, self.ids.clone())
                .map_err(|e| anyhow!("{e}"))?;
        let mut arg = None;

        let not_updated = loop {
            match coroutine.resume(arg.take()) {
                CancelJmapEmailSubmissionsResult::Io(io) => {
                    arg = Some(handle(&mut jmap.stream, io)?)
                }
                CancelJmapEmailSubmissionsResult::Ok { not_updated, .. } => {
                    break not_updated
                }
                CancelJmapEmailSubmissionsResult::Err { err, .. } => bail!(err),
            }
        };

        for (id, err) in &not_updated {
            let mut ctx = anyhow!("Cancel submission `{id}` error");
            if let Some(desc) = &err.description {
                ctx = anyhow!("{desc}").context(ctx);
            }
            if !err.properties.is_empty() {
                let props = err.properties.join(", ");
                ctx = anyhow!("Invalid properties {props}").context(ctx);
            }
            bail!(ctx);
        }

        printer.out(Message::new(format!("{} submission(s) canceled.", self.ids.len())))
    }
}
