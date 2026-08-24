use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    shared::table::style_from_preset,
    sieve::{client::SieveClient, protocol::SieveCapability},
};

/// Query and print the server's current ManageSieve capabilities.
#[derive(Debug, Parser)]
pub struct SieveCapabilityListCommand;

impl SieveCapabilityListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &Account,
        client: &mut SieveClient,
    ) -> Result<()> {
        printer.out(SieveCapabilities {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            capabilities: client.capability()?,
        })
    }
}

/// Structured output for `sieve capability`.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct SieveCapabilities {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    pub capabilities: Vec<SieveCapability>,
}

impl fmt::Display for SieveCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();
        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("CAPABILITY"), Cell::new("VALUES")]));

        for capability in &self.capabilities {
            table.add_row(Row::from([
                Cell::new(&capability.name),
                Cell::new(capability.values.join(" ")),
            ]));
        }

        writeln!(f)?;
        write!(f, "{table}")
    }
}
