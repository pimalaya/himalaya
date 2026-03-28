use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::{
    rfc8621::coroutines::email_submission_set::{
        JmapEmailSubmissionSet, JmapEmailSubmissionSetResult,
    },
    rfc8621::types::email_submission::{
        EmailAddressWithParameters, EmailSubmissionCreate, Envelope,
    },
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{account::JmapAccount, submission::query::SubmissionsTable};

/// Submit a JMAP email for sending (EmailSubmission/set).
///
/// The email must already exist as a draft in the JMAP account.
/// This is the JMAP equivalent of SMTP message submission.
#[derive(Debug, Parser)]
pub struct CreateSubmissionCommand {
    /// The ID of the draft email to send.
    #[arg(value_name = "EMAIL_ID")]
    pub email_id: String,

    /// The identity ID to send as (from `identity get`).
    #[arg(long, value_name = "IDENTITY_ID")]
    pub identity_id: String,

    /// Override the MAIL FROM address (uses `From` header if omitted).
    #[arg(long, value_name = "ADDRESS")]
    pub mail_from: Option<String>,

    /// Override the RCPT TO addresses (uses `To`, `Cc`, `Bcc` if omitted).
    #[arg(long, value_name = "ADDRESS")]
    pub rcpt_to: Vec<String>,
}

impl CreateSubmissionCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let envelope = if let Some(mail_from_addr) = self.mail_from {
            let rcpt_to = self
                .rcpt_to
                .into_iter()
                .map(|addr| EmailAddressWithParameters {
                    email: addr,
                    parameters: None,
                })
                .collect();
            Some(Envelope {
                mail_from: EmailAddressWithParameters {
                    email: mail_from_addr,
                    parameters: None,
                },
                rcpt_to,
            })
        } else {
            None
        };

        let submission = EmailSubmissionCreate {
            identity_id: self.identity_id,
            email_id: self.email_id.clone(),
            envelope,
        };

        let mut submissions = HashMap::new();
        submissions.insert(self.email_id.clone(), submission);

        let mut coroutine =
            JmapEmailSubmissionSet::new(&jmap.session, &jmap.http_auth, submissions)?;
        let mut arg = None;

        let (created, errs) = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailSubmissionSetResult::Io { io } => {
                    arg = Some(handle(&mut jmap.stream, io)?)
                }
                JmapEmailSubmissionSetResult::Ok {
                    created,
                    not_created,
                    ..
                } => break (created, not_created),
                JmapEmailSubmissionSetResult::Err { err, .. } => bail!(err),
            }
        };

        if let Some(err) = errs.get(&self.email_id) {
            let mut ctx = anyhow!("Send email `{}` error", &self.email_id);

            if let Some(desc) = &err.description {
                ctx = anyhow!("{desc}").context(ctx);
            }

            if !err.properties.is_empty() {
                let props = err.properties.join(", ");
                ctx = anyhow!("Invalid properties {props}").context(ctx);
            }

            bail!(ctx);
        }

        let table = SubmissionsTable {
            preset: account.table_preset,
            submissions: created.into_values().collect(),
        };

        printer.out(table)
    }
}
