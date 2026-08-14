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
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailForwardingAddressGet::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?
        };
        let address = out.response;

        printer.out(GmailSettingsForwardingAddressGetOutput(address))
    }
}

/// A Gmail forwarding address, rendered as aligned text or, under
/// `--json`, as the forwarding address resource itself instead of a
/// wrapped human string.
///
/// The resource is emitted verbatim so that one address read with `get`
/// has the very same shape as a row of `list`.
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
