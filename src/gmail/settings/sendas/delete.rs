//! # Gmail send-as delete
//!
//! The `gmail settings sendas delete` command,
//! `users.settings.sendAs.delete`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::send_as::delete::GmailSendAsDelete;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Delete a Gmail send-as alias (settings.sendAs.delete).
#[derive(Debug, Parser)]
pub struct GmailSettingsSendAsDeleteCommand {
    /// E-mail address of the send-as alias to delete.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsSendAsDeleteCommand {
    /// Removes the send-as alias.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailSendAsDelete::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?;
        }

        printer.out(Message::new(format!(
            "Gmail send-as `{}` successfully deleted",
            self.email
        )))
    }
}
