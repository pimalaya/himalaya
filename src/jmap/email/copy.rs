use std::{
    collections::BTreeMap,
    io::{Read, Write},
};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::{
    email::EmailCopy,
    email_copy::{JmapEmailCopy, JmapEmailCopyResult},
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::{account::JmapAccount, error::format_set_error};

const READ_BUFFER_SIZE: usize = 16 * 1024;

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

        let mailbox_ids: BTreeMap<String, bool> =
            self.mailbox_id.into_iter().map(|m| (m, true)).collect();

        let emails: BTreeMap<String, EmailCopy> = self
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
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let not_created = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailCopyResult::Ok { not_created, .. } => break not_created,
                JmapEmailCopyResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapEmailCopyResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapEmailCopyResult::Err(err) => bail!("{err}"),
            }
        };

        if !not_created.is_empty() {
            let mut msg = String::from("Copy JMAP email(s) error");

            for (id, err) in not_created {
                msg.push_str(&format!("\n  `{id}`"));
                msg.push_str(&format_set_error(&err));
            }

            bail!(msg)
        }

        printer.out(Message::new("Email(s) successfully copied"))
    }
}
