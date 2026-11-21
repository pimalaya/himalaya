//! # Gmail forwarding address list
//!
//! The `gmail settings forwarding-addresses list` command,
//! `users.settings.forwardingAddresses.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::forwarding_addresses::GmailForwardingAddress;
use pimalaya_cli::{listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
    unified::backend::Backend,
};

/// List all Gmail forwarding addresses
/// (users.settings.forwardingAddresses.list).
///
/// JSON output: Gmail's answer as Gmail sent it, `{"forwardingAddresses":
/// [<Gmail forwarding address>...]}`, `[]` when there is none.
#[derive(Debug, Parser)]
pub struct GmailSettingsForwardingAddressesListCommand;

impl GmailSettingsForwardingAddressesListCommand {
    /// Lists every forwarding address and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let path = ["settings", "forwardingAddresses"];
        let page = client.raw_list_at(&path, "forwardingAddresses", Vec::new())?;

        printer.out(GmailSettingsForwardingAddressListOutput {
            account: account.name().to_string(),
            forwarding_addresses: page.items,
            rest: page.rest,
        })
    }
}

/// The `gmail settings forwarding-addresses list` output, the Gmail
/// forwarding addresses.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailSettingsForwardingAddressListOutput {
    #[serde(skip)]
    account: String,
    /// The forwarding addresses, each as Gmail sent it.
    #[schemars(with = "Vec<Map<String, Value>>")]
    forwarding_addresses: Vec<Value>,
    /// The rest of Gmail's answer as Gmail sent it.
    #[serde(flatten)]
    rest: Map<String, Value>,
}

impl fmt::Display for GmailSettingsForwardingAddressListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail.as_str()));

        for address in &self.forwarding_addresses {
            let address: GmailForwardingAddress =
                serde_json::from_value(address.clone()).unwrap_or_default();
            let status = address
                .verification_status
                .map(verification_status_wire)
                .unwrap_or_default();

            listing.items.push([
                status.to_string(),
                sanitize::text(&address.forwarding_email).into_owned(),
            ]);
        }

        write!(f, "{listing}")
    }
}
