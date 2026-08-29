//! # Gmail draft delete
//!
//! The `gmail drafts delete` command, `users.drafts.delete`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::drafts::delete::GmailDraftDelete;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Permanently delete a Gmail draft (users.drafts.delete).
#[derive(Debug, Parser)]
pub struct GmailDraftDeleteCommand {
    /// The id of the draft to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailDraftDeleteCommand {
    /// Deletes the draft for good.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailDraftDelete::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?
        };

        printer.out(Message::new(format!(
            "Gmail draft `{}` successfully deleted",
            self.id
        )))
    }
}
