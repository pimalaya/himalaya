use anyhow::{Result, bail};
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::{client::JmapClient, error::format_set_error};

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
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let output = client.email_submission_cancel(self.ids.clone())?;

        if !output.not_updated.is_empty() {
            let mut msg = String::from("Cancel submission(s) error");

            for (id, err) in &output.not_updated {
                msg.push_str(&format!("\n  `{id}`"));
                msg.push_str(&format_set_error(err));
            }

            bail!(msg);
        }

        printer.out(Message::new(format!(
            "{} submission(s) canceled",
            self.ids.len()
        )))
    }
}
