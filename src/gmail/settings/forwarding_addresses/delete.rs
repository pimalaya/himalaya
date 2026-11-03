//! # Gmail forwarding address delete
//!
//! The `gmail settings forwarding-addresses delete` command,
//! `users.settings.forwardingAddresses.delete`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::forwarding_addresses::delete::GmailForwardingAddressDelete;
use pimalaya_cli::printer::Printer;

use crate::{gmail::client::GmailClient, shared::receipt::IdReceipt};

/// Delete a Gmail forwarding address
/// (users.settings.forwardingAddresses.delete).
#[derive(Debug, Parser)]
pub struct GmailSettingsForwardingAddressDeleteCommand {
    /// Email address of the forwarding address to delete.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsForwardingAddressDeleteCommand {
    /// Removes the forwarding address.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailForwardingAddressDelete::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?;
        }

        printer.out(IdReceipt {
            id: self.email,
            done: "Gmail forwarding address successfully deleted",
        })
    }
}
