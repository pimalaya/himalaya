//! # ManageSieve capability
//!
//! The `sieve capability` command, RFC 5804 `CAPABILITY`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, sieve::client::SieveClient};

/// Query and print the server's current ManageSieve capabilities.
#[derive(Debug, Parser)]
pub struct SieveCapabilityListCommand;

impl SieveCapabilityListCommand {
    /// Reads the capabilities the server advertises and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &Account,
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

        printer.out(SieveCapabilityOutput { capabilities })
    }
}

/// One capability the server advertises.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SieveCapability {
    /// The capability name, as the server spelled it.
    pub name: String,
    /// The value, for the capabilities carrying one.
    pub value: Option<String>,
}

/// The `sieve capability` output, the capabilities the server advertised.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SieveCapabilityOutput {
    /// The capabilities the server advertised.
    pub capabilities: Vec<SieveCapability>,
}

impl fmt::Display for SieveCapabilityOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<_> = self
            .capabilities
            .iter()
            .map(|capability| sanitize::text(&capability.name))
            .collect();

        let mut fields = Fields::default();

        for (name, capability) in names.iter().zip(&self.capabilities) {
            let value = capability.value.as_deref().unwrap_or_default();
            fields.push(name, sanitize::text(value));
        }

        write!(f, "{fields}")
    }
}
