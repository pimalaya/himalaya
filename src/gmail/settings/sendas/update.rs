//! # Gmail send-as update
//!
//! The `gmail settings sendas update` command,
//! `users.settings.sendAs.update`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::send_as::{
    GmailSendAs, patch::GmailSendAsPatch, update::GmailSendAsUpdate,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Update a Gmail send-as alias (settings.sendAs.update/patch).
#[derive(Debug, Parser)]
pub struct GmailSettingsSendAsUpdateCommand {
    /// E-mail address of the send-as alias to update.
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
    /// Switch from a full update to a partial patch; without it the
    /// default update clears any field you omit.
    #[arg(long)]
    pub patch: bool,
}

impl GmailSettingsSendAsUpdateCommand {
    /// Applies the patches to the send-as alias.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let send_as = GmailSendAs {
            send_as_email: self.email.clone(),
            display_name: self.display_name,
            reply_to_address: self.reply_to_address,
            signature: self.signature,
            treat_as_alias: self.treat_as_alias.then_some(true),
            ..Default::default()
        };

        if self.patch {
            let c = GmailSendAsPatch::new(&client.auth, &client.user_id, &self.email, &send_as)?;
            client.run(c)?;
        } else {
            let c = GmailSendAsUpdate::new(&client.auth, &client.user_id, &self.email, &send_as)?;
            client.run(c)?;
        }

        printer.out(Message::new(format!(
            "Gmail send-as `{}` successfully updated",
            self.email
        )))
    }
}
