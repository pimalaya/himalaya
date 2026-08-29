//! # Gmail forwarding address get
//!
//! The `gmail settings forwarding-addresses get` command,
//! `users.settings.forwardingAddresses.get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::forwarding_addresses::{
    GmailForwardingAddress, get::GmailForwardingAddressGet,
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::{client::GmailClient, settings::convert::verification_status_wire};

/// Get a Gmail forwarding address by email address
/// (users.settings.forwardingAddresses.get).
#[derive(Debug, Parser)]
pub struct GmailSettingsForwardingAddressGetCommand {
    /// Email address of the forwarding address to get.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsForwardingAddressGetCommand {
    /// Fetches the forwarding address and tables it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailForwardingAddressGet::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?
        };
        let address = out.response;

        printer.out(GmailSettingsForwardingAddressGetOutput(address))
    }
}

/// The `gmail settings forwarding-addresses get` output.
///
/// The resource is emitted verbatim, so one address has the very same
/// shape here as in a `list` row.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsForwardingAddressGetOutput(GmailForwardingAddress);

impl fmt::Display for GmailSettingsForwardingAddressGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Email: {}", self.0.forwarding_email)?;

        if let Some(status) = self.0.verification_status {
            writeln!(f, "Verification: {}", verification_status_wire(status))?;
        }

        Ok(())
    }
}
