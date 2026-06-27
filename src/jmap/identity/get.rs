use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_jmap::rfc8621::identity::{JmapIdentity, get::JmapIdentityGetOptions};
use log::warn;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::jmap::client::JmapClient;

/// Get JMAP identities (Identity/get).
///
/// Lists sender identities available for sending email. Pass no IDs to
/// list all identities.
#[derive(Debug, Parser)]
pub struct JmapIdentityGetCommand {
    /// Identity ID(s) to retrieve (omit to get all).
    #[arg(value_name = "ID")]
    pub ids: Option<Vec<String>>,
}

impl JmapIdentityGetCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = client.identity_get(JmapIdentityGetOptions { ids: self.ids })?;

        for id in output.not_found {
            warn!("identity `{id}` not found");
        }

        let table = IdentitiesTable {
            preset: account.table_preset().to_string(),
            identities: output.identities,
        };

        printer.out(table)
    }
}

/// Renderable table of sender identities.
#[derive(Clone, Debug, Serialize)]
pub struct IdentitiesTable {
    #[serde(skip)]
    pub preset: String,
    pub identities: Vec<JmapIdentity>,
}

impl fmt::Display for IdentitiesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("NAME"),
                Cell::new("EMAIL"),
            ]))
            .add_rows(
                self.identities.iter().map(|i| {
                    Row::from([Cell::new(&i.id), Cell::new(&i.name), Cell::new(&i.email)])
                }),
            );

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
