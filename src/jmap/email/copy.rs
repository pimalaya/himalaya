use std::collections::HashMap;

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::{
    rfc8621::coroutines::email_copy::{JmapEmailCopy, JmapEmailCopyResult},
    rfc8621::types::email::EmailCopy,
};
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Copy JMAP emails from another account (Email/copy).
#[derive(Debug, Parser)]
pub struct JmapEmailCopyCommand {
    /// Email ID(s) to copy.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,

    /// Source account ID to copy from.
    #[arg(long, value_name = "ACCOUNT-ID")]
    pub from_account: String,

    /// Destination mailbox ID(s) to place copies in.
    #[arg(long, value_name = "MAILBOX-ID", required = false)]
    pub mailbox_id: Vec<String>,
}

impl JmapEmailCopyCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mailbox_ids: HashMap<String, bool> =
            self.mailbox_id.into_iter().map(|m| (m, true)).collect();

        let emails: HashMap<String, EmailCopy> = self
            .ids
            .into_iter()
            .map(|id| {
                (
                    id.clone(),
                    EmailCopy {
                        id,
                        mailbox_ids: mailbox_ids.clone(),
                        keywords: None,
                        received_at: None,
                    },
                )
            })
            .collect();

        let mut coroutine = JmapEmailCopy::new(
            &jmap.session,
            &jmap.http_auth,
            self.from_account.clone(),
            emails,
        )?;
        let mut arg = None;

        let not_created = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailCopyResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapEmailCopyResult::Ok { not_created, .. } => break not_created,
                JmapEmailCopyResult::Err { err, .. } => bail!(err),
            }
        };

        if !not_created.is_empty() {
            let mut msg = String::from("Copy JMAP email(s) error");

            for (id, err) in not_created {
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

            bail!(msg)
        }

        printer.out(Message::new("Email(s) successfully copied"))
    }
}
