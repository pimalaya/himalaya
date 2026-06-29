use std::collections::BTreeMap;

use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::email_submission::{
    JmapEmailAddressWithParameters, JmapEmailSubmissionCreate, JmapEnvelope,
};
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::jmap::{
    client::JmapClient, error::format_set_error, submission::query::SubmissionsTable,
};

/// Submit a JMAP email for sending (EmailSubmission/set).
///
/// The email must already exist as a draft in the JMAP account.
/// This is the JMAP equivalent of SMTP message submission.
#[derive(Debug, Parser)]
pub struct JmapSubmissionCreateCommand {
    /// The ID of the draft email to send.
    #[arg(value_name = "EMAIL-ID")]
    pub email_id: String,

    /// The identity ID to send as (from `identity get`).
    #[arg(long, value_name = "IDENTITY-ID")]
    pub identity_id: String,

    /// Override the MAIL FROM address (uses `From` header if omitted).
    #[arg(long, value_name = "ADDRESS")]
    pub mail_from: Option<String>,

    /// Override the RCPT TO addresses (uses `To`, `Cc`, `Bcc` if omitted).
    #[arg(long, value_name = "ADDRESS")]
    pub rcpt_to: Vec<String>,
}

impl JmapSubmissionCreateCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        // The JMAP envelope is all-or-nothing: mail_from is required, so
        // it cannot be derived while overriding rcpt_to (or vice versa).
        // With neither, the server derives mail_from from the identity
        // and rcpt_to from the message headers.
        let envelope = match (self.mail_from, self.rcpt_to.is_empty()) {
            (None, true) => None,
            (Some(mail_from_addr), false) => {
                let rcpt_to = self
                    .rcpt_to
                    .into_iter()
                    .map(|addr| JmapEmailAddressWithParameters {
                        email: addr,
                        parameters: None,
                    })
                    .collect();
                Some(JmapEnvelope {
                    mail_from: JmapEmailAddressWithParameters {
                        email: mail_from_addr,
                        parameters: None,
                    },
                    rcpt_to,
                })
            }
            _ => bail!("Overriding the JMAP envelope requires both --mail-from and --rcpt-to"),
        };

        let submission = JmapEmailSubmissionCreate {
            identity_id: self.identity_id,
            email_id: self.email_id.clone(),
            envelope,
        };

        let mut submissions = BTreeMap::new();
        submissions.insert(self.email_id.clone(), submission);

        let output = client.email_submission_set(submissions)?;

        if let Some(err) = output.not_created.get(&self.email_id) {
            let mut msg = format!("Send email `{}` error", self.email_id);
            msg.push_str(&format_set_error(err));
            bail!(msg);
        }

        let table = SubmissionsTable {
            preset: account.table_preset().to_string(),
            submissions: output.created.into_values().collect(),
        };

        printer.out(table)
    }
}
