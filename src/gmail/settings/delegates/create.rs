use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::delegates::{GmailDelegate, create::GmailDelegateCreate};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Create a Gmail delegate (users.settings.delegates.create).
#[derive(Debug, Parser)]
pub struct GmailSettingsDelegateCreateCommand {
    /// Email address of the delegate to create.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsDelegateCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let delegate = GmailDelegate {
            delegate_email: self.email.clone(),
            verification_status: None,
        };

        let out = {
            let c = GmailDelegateCreate::new(&client.auth, &client.user_id, &delegate)?;
            client.run(c)?
        };
        let created = out.response;

        printer.out(Message::new(format!(
            "Gmail delegate `{}` successfully created",
            created.delegate_email
        )))
    }
}
