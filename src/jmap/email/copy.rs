use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::{
    coroutines::email_copy::{CopyJmapEmails, CopyJmapEmailsResult},
    types::email::EmailCopy,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Copy JMAP emails from another account (Email/copy).
#[derive(Debug, Parser)]
pub struct CopyEmailCommand {
    /// Email ID(s) to copy.
    #[arg(value_name = "EMAIL-ID", required = true, num_args = 1..)]
    pub ids: Vec<String>,

    /// Source account ID to copy from.
    #[arg(long, value_name = "ACCOUNT-ID")]
    pub from_account: String,

    /// Destination mailbox ID(s) to place copies in.
    #[arg(long, value_name = "MAILBOX-ID", num_args = 0..)]
    pub mailbox_id: Vec<String>,
}

impl CopyEmailCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mailbox_ids: HashMap<String, bool> =
            self.mailbox_id.iter().map(|m| (m.clone(), true)).collect();

        let emails: HashMap<String, EmailCopy> = self
            .ids
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    EmailCopy {
                        id: id.clone(),
                        mailbox_ids: mailbox_ids.clone(),
                        keywords: None,
                        received_at: None,
                    },
                )
            })
            .collect();

        let mut coroutine = CopyJmapEmails::new(jmap.context, self.from_account.clone(), emails)?;
        let mut arg = None;

        let not_created = loop {
            match coroutine.resume(arg.take()) {
                CopyJmapEmailsResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                CopyJmapEmailsResult::Ok { not_created, .. } => break not_created,
                CopyJmapEmailsResult::Err { err, .. } => bail!(err),
            }
        };

        for (id, err) in &not_created {
            let mut ctx = anyhow!("Failed to copy email `{id}`");

            if let Some(desc) = &err.description {
                ctx = anyhow!(desc.clone()).context(ctx);
            }

            bail!(ctx);
        }

        printer.out(Message::new("Email(s) successfully copied"))
    }
}
