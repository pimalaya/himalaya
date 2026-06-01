// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::email_submission::{
    EmailAddressWithParameters, EmailSubmissionCreate, Envelope,
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

impl JmapSubmissionCreateCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
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
