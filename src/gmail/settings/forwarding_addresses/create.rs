use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::forwarding_addresses::{
    GmailForwardingAddress, create::GmailForwardingAddressCreate,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Create a Gmail forwarding address
/// (users.settings.forwardingAddresses.create).
#[derive(Debug, Parser)]
pub struct GmailSettingsForwardingAddressCreateCommand {
    /// Email address of the forwarding address to create.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsForwardingAddressCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let address = GmailForwardingAddress {
            forwarding_email: self.email.clone(),
            verification_status: None,
        };

        let out = {
            let c = GmailForwardingAddressCreate::new(&client.auth, &client.user_id, &address)?;
            client.run(c)?
        };
        let created = out.response;

        printer.out(Message::new(format!(
            "Gmail forwarding address `{}` successfully created",
            created.forwarding_email
        )))
    }
}
