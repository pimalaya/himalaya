//! # Gmail send-as verify
//!
//! The `gmail settings sendas verify` command,
//! `users.settings.sendAs.verify`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::send_as::verify::GmailSendAsVerify;
use pimalaya_cli::printer::Printer;

use crate::{gmail::client::GmailClient, shared::receipt::IdReceipt};

/// Send a verification e-mail for a Gmail send-as alias
/// (settings.sendAs.verify).
#[derive(Debug, Parser)]
pub struct GmailSettingsSendAsVerifyCommand {
    /// E-mail address of the send-as alias to verify.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsSendAsVerifyCommand {
    /// Asks Gmail to send the alias its verification mail.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailSendAsVerify::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?;
        }

        printer.out(IdReceipt {
            id: self.email,
            done: "Verification e-mail sent for Gmail send-as",
        })
    }
}
