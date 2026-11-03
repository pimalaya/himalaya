//! # ManageSieve deactivate
//!
//! The `sieve deactivate` command, an RFC 5804 `SETACTIVE` naming no
//! script.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::sieve::client::SieveClient;

/// Disable the currently active server-side Sieve script.
///
/// Filtering stops until a script is activated again; doing it twice is
/// not an error.
#[derive(Debug, Parser)]
pub struct SieveScriptDeactivateCommand;

impl SieveScriptDeactivateCommand {
    /// Leaves the account with no active script.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        client.activate_script(None)?;
        printer.out(SieveDeactivateOutput { active: None })
    }
}

/// The `sieve deactivate` output.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SieveDeactivateOutput {
    /// The active script after the command, always `null`.
    active: Option<String>,
}

impl fmt::Display for SieveDeactivateOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Sieve script successfully deactivated")
    }
}
