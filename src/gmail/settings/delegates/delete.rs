//! # Gmail delegate delete
//!
//! The `gmail settings delegates delete` command,
//! `users.settings.delegates.delete`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::delegates::delete::GmailDelegateDelete;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Delete a Gmail delegate (users.settings.delegates.delete).
#[derive(Debug, Parser)]
pub struct GmailSettingsDelegateDeleteCommand {
    /// Email address of the delegate to delete.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsDelegateDeleteCommand {
    /// Removes the delegate.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailDelegateDelete::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?;
        }

        printer.out(Message::new(format!(
            "Gmail delegate `{}` successfully deleted",
            self.email
        )))
    }
}
