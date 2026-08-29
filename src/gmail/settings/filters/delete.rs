//! # Gmail filter delete
//!
//! The `gmail settings filters delete` command,
//! `users.settings.filters.delete`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::filters::delete::GmailFilterDelete;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Delete a Gmail filter (users.settings.filters.delete).
#[derive(Debug, Parser)]
pub struct GmailSettingsFilterDeleteCommand {
    /// Identifier of the filter to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailSettingsFilterDeleteCommand {
    /// Deletes the filter.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailFilterDelete::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?;
        }

        printer.out(Message::new(format!(
            "Gmail filter `{}` successfully deleted",
            self.id
        )))
    }
}
