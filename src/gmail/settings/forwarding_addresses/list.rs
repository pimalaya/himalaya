use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::settings::forwarding_addresses::list::{
    GmailForwardingAddressesList, GmailForwardingAddressesListResponse,
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
    shared::table::style_from_preset,
};

/// List all Gmail forwarding addresses
/// (users.settings.forwardingAddresses.list).
#[derive(Debug, Parser)]
pub struct GmailSettingsForwardingAddressesListCommand;

impl GmailSettingsForwardingAddressesListCommand {
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

        let table = ForwardingAddressesTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            response: resp,
        };

        printer.out(table)
    }
}

/// Renderable table of Gmail forwarding addresses.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ForwardingAddressesTable {
    #[serde(skip)]
    #[schemars(skip)]
    preset: String,
    #[serde(skip)]
    #[schemars(skip)]
    arrangement: ContentArrangement,
    response: GmailForwardingAddressesListResponse,
}

impl fmt::Display for ForwardingAddressesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("EMAIL"), Cell::new("VERIFICATION")]))
            .add_rows(self.response.forwarding_addresses.iter().map(|address| {
                let status = address
                    .verification_status
                    .map(verification_status_wire)
                    .unwrap_or_default();

                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&address.forwarding_email).fg(Color::Reset))
                    .add_cell(Cell::new(status).fg(Color::Reset));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
