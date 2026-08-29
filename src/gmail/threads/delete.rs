//! # Gmail thread delete
//!
//! The `gmail threads delete` command, `users.threads.delete`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::threads::delete::GmailThreadDelete;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Permanently delete a Gmail thread (users.threads.delete).
#[derive(Debug, Parser)]
pub struct GmailThreadDeleteCommand {
    /// The id of the thread to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailThreadDeleteCommand {
    /// Deletes the thread and its messages for good.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailThreadDelete::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?
        };

        printer.out(Message::new(format!(
            "Gmail thread `{}` permanently deleted",
            self.id
        )))
    }
}
