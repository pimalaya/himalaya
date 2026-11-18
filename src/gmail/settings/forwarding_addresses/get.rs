//! # Gmail forwarding address get
//!
//! The `gmail settings forwarding-addresses get` command,
//! `users.settings.forwardingAddresses.get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::forwarding_addresses::GmailForwardingAddress;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

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
    /// Fetches the forwarding address and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let address = client.raw_get(&["settings", "forwardingAddresses", &self.email])?;
        printer.out(GmailSettingsForwardingAddressGetOutput(Value::Object(
            address,
        )))
    }
}

/// The `gmail settings forwarding-addresses get` output.
///
/// The resource is emitted verbatim, so one address has the very same
/// shape here as in a `list` row.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsForwardingAddressGetOutput(
    #[schemars(with = "Map<String, Value>")] Value,
);

impl fmt::Display for GmailSettingsForwardingAddressGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let address: GmailForwardingAddress =
            serde_json::from_value(self.0.clone()).unwrap_or_default();
        let mut fields = Fields::default();
        fields.push("Email", sanitize::text(&address.forwarding_email));

        if let Some(status) = address.verification_status {
            fields.push("Verification", verification_status_wire(status));
        }

        write!(f, "{fields}")
    }
}
