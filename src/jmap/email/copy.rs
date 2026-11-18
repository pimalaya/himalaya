//! # JMAP email copy
//!
//! The `jmap email copy` command, RFC 8621 `Email/copy`.

use std::collections::BTreeMap;

use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::email::copy::JmapEmailCopyArgs;
use pimalaya_cli::printer::Printer;

use crate::{
    error::ErrorCode,
    jmap::{client::JmapClient, set::JmapSetReceipt},
};

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
    /// Copies the emails and reports any the server refused.
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

        let ids = output
            .created
            .into_values()
            .filter_map(|email| email.id)
            .collect();

        JmapSetReceipt::new(
            "JMAP emails successfully copied",
            ids,
            output.not_created,
            Some(ErrorCode::MessageNotFound),
        )
        .print(printer)
    }
}
