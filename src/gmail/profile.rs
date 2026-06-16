use anyhow::Result;
use clap::{Parser, Subcommand};
use pimalaya_cli::printer::{Message, Printer};

use crate::{account::context::Account, gmail::client::GmailClient};

/// Manage the Gmail user profile (users.getProfile).
#[derive(Debug, Subcommand)]
pub enum GmailProfileCommand {
    Get(GmailProfileGetCommand),
}

impl GmailProfileCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
        }
    }
}

/// Get the Gmail profile: email address, message/thread totals and the
/// current history id.
#[derive(Debug, Parser)]
pub struct GmailProfileGetCommand;

impl GmailProfileGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let profile = client.profile_get()?.response;

        let mut out = String::new();
        out.push_str(&format!("Email: {}\n", profile.email_address));
        if let Some(total) = profile.messages_total {
            out.push_str(&format!("Messages: {total}\n"));
        }
        if let Some(total) = profile.threads_total {
            out.push_str(&format!("Threads: {total}\n"));
        }
        if let Some(history_id) = profile.history_id {
            out.push_str(&format!("History id: {history_id}\n"));
        }

        printer.out(Message::new(out))
    }
}
