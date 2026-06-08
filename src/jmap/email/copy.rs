use std::collections::BTreeMap;

use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::email::JmapEmailCopyArgs;
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::{client::JmapClient, error::format_set_error};

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
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let mailbox_ids: BTreeMap<String, bool> =
            self.mailbox_id.into_iter().map(|m| (m, true)).collect();

        let emails: BTreeMap<String, JmapEmailCopyArgs> = self
            .ids
            .into_iter()
            .map(|id| {
                (
                    id.clone(),
                    JmapEmailCopyArgs {
                        id,
                        mailbox_ids: mailbox_ids.clone(),
                        keywords: None,
                        received_at: None,
                    },
                )
            })
            .collect();

        let output = client.email_copy(self.from_account.clone(), emails)?;

        if !output.not_created.is_empty() {
            let mut msg = String::from("Copy JMAP email(s) error");

            for (id, err) in output.not_created {
                msg.push_str(&format!("\n  `{id}`"));
                msg.push_str(&format_set_error(&err));
            }

            bail!(msg)
        }

        printer.out(Message::new("Email(s) successfully copied"))
    }
}
