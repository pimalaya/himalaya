//! # Gmail profile get
//!
//! The `gmail profile get` command, `users.getProfile`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::client::GmailClient;

/// Get the Gmail profile: email address, message/thread totals and the
/// current history id.
#[derive(Debug, Parser)]
pub struct GmailProfileGetCommand;

impl GmailProfileGetCommand {
    /// Fetches the profile and tables it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let profile = client.profile_get()?.response;

        printer.out(GmailProfileOutput {
            email: profile.email_address,
            messages_total: profile.messages_total,
            threads_total: profile.threads_total,
            history_id: profile.history_id,
        })
    }
}

/// Gmail profile, rendered as aligned text or, under `--json`, as a
/// structured object (`{email, messages-total, threads-total,
/// history-id}`) instead of a wrapped human string.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailProfileOutput {
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    messages_total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    threads_total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    history_id: Option<String>,
}

impl fmt::Display for GmailProfileOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Email: {}", self.email)?;
        if let Some(total) = self.messages_total {
            writeln!(f, "Messages: {total}")?;
        }
        if let Some(total) = self.threads_total {
            writeln!(f, "Threads: {total}")?;
        }
        if let Some(history_id) = &self.history_id {
            writeln!(f, "History id: {history_id}")?;
        }
        Ok(())
    }
}
