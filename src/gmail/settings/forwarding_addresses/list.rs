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
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
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
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let out = {
            let c = GmailForwardingAddressesList::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let resp = out.response;

        printer.out(GmailSettingsForwardingAddressListOutput { response: resp })
    }
}

/// The `gmail settings forwarding-addresses list` output, the Gmail
/// forwarding addresses.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailSettingsForwardingAddressListOutput {
    #[serde(flatten)]
    response: GmailForwardingAddressesListResponse,
}

impl fmt::Display for GmailSettingsForwardingAddressListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::new(["EMAIL", "VERIFICATION"]);

        for address in &self.response.forwarding_addresses {
            let status = address
                .verification_status
                .map(verification_status_wire)
                .unwrap_or_default();

            columns.push([&*sanitize::text(&address.forwarding_email), status]);
        }

        write!(f, "{columns}")
    }
}
