//! # Gmail forwarding address list
//!
//! The `gmail settings forwarding-addresses list` command,
//! `users.settings.forwardingAddresses.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::forwarding_addresses::list::{
    GmailForwardingAddressesList, GmailForwardingAddressesListResponse,
};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
    listing::Listing,
};

/// List all Gmail forwarding addresses
/// (users.settings.forwardingAddresses.list).
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
        let out = {
            let c = GmailForwardingAddressesList::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let resp = out.response;

        printer.out(GmailSettingsForwardingAddressListOutput {
            account: account.name().to_string(),
            response: resp,
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
    #[serde(flatten)]
    response: GmailForwardingAddressesListResponse,
}

impl fmt::Display for GmailSettingsForwardingAddressListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail));

        for address in &self.response.forwarding_addresses {
            let status = address
                .verification_status
                .map(verification_status_wire)
                .unwrap_or_default();

            listing
                .items
                .push([status, &*sanitize::text(&address.forwarding_email)]);
        }

        write!(f, "{listing}")
    }
}
