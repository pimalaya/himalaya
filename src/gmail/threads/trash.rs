//! # Gmail thread trash
//!
//! The `gmail threads trash` command, `users.threads.trash`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::threads::trash::GmailThreadTrash;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Move a Gmail thread to the trash (users.threads.trash).
#[derive(Debug, Parser)]
pub struct GmailThreadTrashCommand {
    /// The id of the thread to trash.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailThreadTrashCommand {
    /// Moves the thread to the trash.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailThreadTrash::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?
        };
        let thread = out.response;

        printer.out(Message::new(format!(
            "Gmail thread `{}` successfully trashed",
            thread.id
        )))
    }
}
