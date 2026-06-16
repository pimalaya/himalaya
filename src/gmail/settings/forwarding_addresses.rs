use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::settings::forwarding_addresses::{
    GmailForwardingAddress, create::GmailForwardingAddressCreate,
    delete::GmailForwardingAddressDelete, get::GmailForwardingAddressGet,
    list::GmailForwardingAddressesList, list::GmailForwardingAddressesListResponse,
};
use pimalaya_cli::printer::{Message, Printer};
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
};

/// Manage Gmail forwarding addresses (users.settings.forwardingAddresses).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsForwardingAddressesCommand {
    List(List),
    Get(Get),
    Create(Create),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(Delete),
}

impl GmailSettingsForwardingAddressesCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}

/// List all Gmail forwarding addresses
/// (users.settings.forwardingAddresses.list).
#[derive(Debug, Parser)]
pub struct List;

impl List {
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

/// Get a Gmail forwarding address by email address
/// (users.settings.forwardingAddresses.get).
#[derive(Debug, Parser)]
pub struct Get {
    /// Email address of the forwarding address to get.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl Get {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailForwardingAddressGet::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?
        };
        let address = out.response;

        let mut text = format!("Email: {}\n", address.forwarding_email);
        if let Some(status) = address.verification_status {
            text.push_str(&format!(
                "Verification: {}\n",
                verification_status_wire(status)
            ));
        }

        printer.out(Message::new(text))
    }
}

/// Create a Gmail forwarding address
/// (users.settings.forwardingAddresses.create).
#[derive(Debug, Parser)]
pub struct Create {
    /// Email address of the forwarding address to create.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl Create {
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

/// Delete a Gmail forwarding address
/// (users.settings.forwardingAddresses.delete).
#[derive(Debug, Parser)]
pub struct Delete {
    /// Email address of the forwarding address to delete.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl Delete {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailForwardingAddressDelete::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?;
        }

        printer.out(Message::new(format!(
            "Gmail forwarding address `{}` successfully deleted",
            self.email
        )))
    }
}

/// Renderable table of Gmail forwarding addresses.
#[derive(Clone, Debug, Serialize)]
pub struct ForwardingAddressesTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    response: GmailForwardingAddressesListResponse,
}

impl fmt::Display for ForwardingAddressesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
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
