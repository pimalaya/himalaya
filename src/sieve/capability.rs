use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, shared::table::style_from_preset, sieve::client::SieveClient,
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
        let capabilities = client
            .capability()?
            .capabilities
            .into_iter()
            .map(|capability| SieveCapability {
                name: capability.name,
                value: capability.value,
            })
            .collect();

        printer.out(SieveCapabilities {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            capabilities,
        })
    }
}

/// One capability the server advertises.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SieveCapability {
    /// The capability name, as the server spelled it.
    pub name: String,
    /// The value, for the capabilities carrying one.
    pub value: Option<String>,
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
            .set_header(Row::from([Cell::new("CAPABILITY"), Cell::new("VALUE")]));

        for capability in &self.capabilities {
            table.add_row(Row::from([
                Cell::new(&capability.name),
                Cell::new(capability.value.as_deref().unwrap_or_default()),
            ]));
        }

        writeln!(f)?;
        write!(f, "{table}")
    }
}
