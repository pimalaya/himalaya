//! # Gmail thread untrash
//!
//! The `gmail threads untrash` command, `users.threads.untrash`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::threads::untrash::GmailThreadUntrash;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Remove a Gmail thread from the trash (users.threads.untrash).
#[derive(Debug, Parser)]
pub struct GmailThreadUntrashCommand {
    /// The id of the thread to untrash.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailThreadUntrashCommand {
    /// Takes the thread back out of the trash.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailThreadUntrash::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?
        };
        let thread = out.response;

        printer.out(Message::new(format!(
            "Gmail thread `{}` successfully untrashed",
            thread.id
        )))
    }
}
