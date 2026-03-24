use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::{
    coroutines::email_submission_set::{SubmitJmapEmail, SubmitJmapEmailResult},
    types::email_submission::{EmailAddressWithParameters, EmailSubmissionCreate, Envelope},
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::account::JmapAccount;

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
                .map(|addr| EmailAddressWithParameters { email: addr, parameters: None })
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

        let mut submissions = std::collections::HashMap::new();
        submissions.insert("send-1".to_string(), submission);

        let mut coroutine = SubmitJmapEmail::new(jmap.context, submissions)?;
        let mut arg = None;

        loop {
            match coroutine.resume(arg.take()) {
                SubmitJmapEmailResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                SubmitJmapEmailResult::Ok { context, not_created, .. } => {
                    jmap.context = context;

                    if let Some(err) = not_created.get("send-1") {
                        bail!(
                            "failed to send email `{}`: {} — {}",
                            self.email_id,
                            err.error_type,
                            err.description.as_deref().unwrap_or("no description")
                        );
                    }

                    break;
                }
                SubmitJmapEmailResult::Err { err, .. } => bail!(err),
            }
        }

        printer.log(format!("Email `{}` successfully sent.", self.email_id))
    }
}
