use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::send_as::verify::GmailSendAsVerify;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Send a verification e-mail for a Gmail send-as alias
/// (settings.sendAs.verify).
#[derive(Debug, Parser)]
pub struct GmailSettingsSendAsVerifyCommand {
    /// E-mail address of the send-as alias to verify.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsSendAsVerifyCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailSendAsVerify::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?;
        }

        printer.out(Message::new(format!(
            "Verification e-mail sent for Gmail send-as `{}`",
            self.email
        )))
    }
}
