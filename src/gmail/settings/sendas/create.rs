use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::send_as::{GmailSendAs, create::GmailSendAsCreate};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Create a Gmail send-as alias (settings.sendAs.create).
#[derive(Debug, Parser)]
pub struct GmailSettingsSendAsCreateCommand {
    /// E-mail address of the send-as alias to create.
    #[arg(value_name = "EMAIL")]
    pub email: String,
    /// Display name shown in the From header for this alias.
    #[arg(long)]
    pub display_name: Option<String>,
    /// Reply-To address to set on messages sent from this alias.
    #[arg(long)]
    pub reply_to_address: Option<String>,
    /// HTML signature appended to messages sent from this alias.
    #[arg(long)]
    pub signature: Option<String>,
    /// Treat this alias as an alias of the primary address.
    #[arg(long)]
    pub treat_as_alias: bool,
}

impl GmailSettingsSendAsCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let send_as = GmailSendAs {
            send_as_email: self.email.clone(),
            display_name: self.display_name,
            reply_to_address: self.reply_to_address,
            signature: self.signature,
            treat_as_alias: self.treat_as_alias.then_some(true),
            ..Default::default()
        };

        let out = {
            let c = GmailSendAsCreate::new(&client.auth, &client.user_id, &send_as)?;
            client.run(c)?
        };
        let created = out.response;

        printer.out(Message::new(format!(
            "Gmail send-as `{}` successfully created",
            created.send_as_email
        )))
    }
}
